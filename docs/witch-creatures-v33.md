# Witch summons and new ordinary species — simulation revision33

## Designer-authorized mechanics

New games and deep-start playtests use revision33. Explicit revisions29–32 still
use their original generation, population caches, stats, Witch curses and spell
behavior. Existing actor state is never retuned on load. Revision33 is admitted by
save validation, explicit-fixture replay and seeded replay. The frozen original
population, placement, reward and numeric catalogs remain intact.

* Witches retain the existing Slowed / Hemophilia / Poisoned cycle. When a valid
  observed enemy is present, a ready Witch prefers to summon one creature into a
  free adjacent land cell. The type is a serialized-RNG 50/50 Night Crawler / Skin
  Walker choice. Cost:10 Energy. Successful casts set `monsterSummonReadyAt` to
  world tick+10; blocked or unaffordable casts neither spend Witch Energy, draw RNG nor
  start the timer. Haste does not advance that absolute deadline. Failed summons
  leave the existing curse/retreat decision available. Uncast cancellation consumes
  its normal player payment and owns that attempted action, without a summon
  cooldown or a second curse attempt. Heartless parents produce
  heartless children. Events deliberately say only “a creature,” not a disguised
  Skin Walker's hidden identity.
* Night Crawlers naturally appear on floors24–34. They use the existing life-drain
  melee trait, with no additional venom, grapple or armor-ignore mechanics.
* Skin Walkers naturally appear on floors26–36. Each actor stores a fixed shallow
  disguise index at construction (`skinWalkerDisguise`), based on its stable ID;
  there is no per-frame or gameplay-RNG disguise draw. Choices are Rat, Blorb,
  Cave Bat, Shellback Beetle, Hemogoblin and Needleling. A living observing player
  body within Chebyshev distance2 reveals the true form, including an active
  Half Here body. Moving outside that range restores the same disguise. A wall
  does not let proximity manufacture observation. Telepathy retains the disguise
  unless a body can actually observe it nearby. No true statistics, hostility or
  combat role change with appearance.
* The map glyph/color/entity key, creature list, hover and inspector share a
  detached appearance projection. Inspector attack, heart and ability previews
  use the same projected appearance instead of re-fetching the true actor by ID.
  Visibility gates remain before projection; hidden actors never become visible.
  Current resource ratios can be displayed on the apparent species' maxima,
  avoiding a diagnostic true-stat leak. The underlying simulation is untouched.

## Explicit new numeric/reward choices

Accuracy/Dodge below includes the separately approved final revision33 flavor
overlay. The new-creature base JSON remains a construction input; the effective
constructor and separate Witch newborn overlay are authoritative.

| Actor | HP | Offense | Defense | Accuracy | Dodge | DR |
|---|---:|---:|---:|---:|---:|---:|
| Natural Night Crawler |292|200|32|144|74|0|
| Natural Skin Walker |376|310|46|148|92|0|
| Witch-summoned Night Crawler |244|200|24|140|64|0|
| Witch-summoned Skin Walker |244|200|24|144|78|0|

All four profiles have50 maximum Energy and45 Perception. Night Crawlers retain
life drain when summoned. Both species grant2 blessings and25 Energy from a
heart; Night Crawler hearts add2 Offense, Skin Walker hearts add2 Perception.
These are explicit provisional choices, not fallback rewards. Natural and
summoned statistics are fixed at construction and serialized without rescaling.
The new ordinary species use the existing introduction weighting (0.2 initially,
+0.2 per floor capped at1), encounter sizes, exact-heart budget and family recipe
rules. They are not inserted into legacy revision pools.

The bestiary adds two pages and retains every original authored Natural History
string. The two new histories are exactly the designer's quotations. Art uses
separate `night-crawler` / `skin-walker` PNGs; no existing creature is reused as a
finished illustration. Both accepted authored RGBA assets are integrated unchanged;
the Skin Walker illustration uses the designer-approved localized pelvic blur. Asset
completeness derives from the actual authored/runtime sets, not obsolete81/79
hard-coded counts.

## Presentation integration for the other revision33 hooks

Hallucinogenic exposure uses the shared detached creature projection. A pure hash
of seed, world tick and already-observed actor ID chooses apparent identities;
frame time, rendering frequency and gameplay RNG do not participate. Illusions
change neither true stats nor allegiance. Greater Breather prose explicitly says
hallucinogenic gas **instead of** caustic gas, while retaining its independently
existing caustic-wound trait. Screamer prose describes the new Disoriented
Accuracy loss instead of the old Offense penalty. Legacy UI fixtures retain the
old wording via the revision gate.

A present `waterBlood` hazard value (including0) marks revision33 water. Clean
water is dark blue-gray, positive concentration adds a concentration-sensitive
red tint; mild3 and heavy100 are visually distinct. Remembered cells use saved
hazard snapshots. Ichor remains separate and overrides the blood treatment.
Explicit water palette edits remain honored. Labels state the concentration and
whether the evidence is current or remembered.

## Verification and limitations

`witch_skinwalker_v33` has11 passing tests covering native successful/failed casts, both offspring
choices, heartless inheritance, curses between summons, legacy Witch behavior,
late population windows and budgets, first-class heart rewards, diagonal range,
Half Here and LOS, corpse-revival identity, Unspeak cost, single Uncast
cancellation, save/load, Revert, See the Future, replay and offscreen catch-up.
`creature_presentation::tests` has7 passing tests covering map/list/hover/inspector consistency,
non-mutating projection, telepathy/fog, deterministic hallucinations, and clean /
mild / heavily bloodied water.

Targeted real-dispatch numeric observations live in
`tools/monster-balance/r33-targeted/`. They use the frozen revision32 joint
ordinary-Unblessed entry player profiles at floors24/26/27, not a claim of a
complete revision33 progression refit. Three-cast Witch payload tests execute
three real successful casts at ticks0/10/20, remove the parent, and fight its
children; they are an isolated payload check, not twenty turns of parent combat.
The whole Witch chase scenario can be censored by controller retreat/cooldown
behavior; censored runs are not a completed-lifespan balance estimate. No prior
77-creature numeric fit was rerun or silently changed.

### Pre-matchup-tuning targeted checkpoint (128 deterministic seeds per case)

* Night Crawler, adjacent at floor24 reference:35.54% mean gross combat damage;
  128 kills,0 deaths,0 censored.
* Skin Walker, adjacent at floor26 reference:33.65%;128 kills,0 deaths,0 censored.
* Three Witch casts at floor27 reference:40.83% isolated offspring payload;
  128 completed payloads,0 deaths,0 censored.
* Whole Witch chase:92 kills,36 player deaths and92 censored aftermaths. This is
  deliberately **not** reported as a reliable completed-lifespan fit.

All512 native trials executed without unsupported-behavior errors. An integrated
rerun at2026-10-26 18:06 UTC, after feeding/Borge/Nest/Zillennia core changes,
produced the same four summaries; see `integrated-results.jsonl` and
`integrated-summary.json`. These remain a checkpoint before the separately
authorized Accuracy/Dodge matchup retuning, not a final revision33 balance fit.


## Adjacent reference integration: Queen and Borge

The separately authorized Saidribee Queen uses an explicit revision33 template:
HP430, Energy100, Offense230, Defense48, Accuracy145, Dodge27, DR0,
Perception55. It is conditional Nest content only, never an ordinary roster
candidate. Its heart reuses the existing Saidribee Guardian reward unchanged
(3 blessings, +1 DR, +3 Defense, 30 energy), with only the Queen identity changed.
Its short Natural History is approved factual copy, not a verbatim designer quote.
Queen summons/Nest generation are covered by their own subsystem tests.

The Borge's visible mark appends to the already-projected creature name in map,
list, hover and inspector. It cannot bypass actor visibility or disclose a Skin
Walker's true species. Marked hearts display their altar-depth provenance while
retaining all original rewards. The Shard offers Use on…, main/offhand equipment,
and Drop, but no Throw. Targeting only admits observed living creatures before
asking core for adjacency/geometry validation. Invalid UI clicks preserve the
selection and do not dispatch a command. Escape cancels through the existing
pending-item flow. The shard's description quotes its authored core description
and suppresses misleading ordinary zero-damage weapon previews.

### Added UI costs

Borge label marking scans at most F<=40 floor metadata records per observed actor;
for O observed actors, total O(O*F) extra work and O(1) scratch excluding formatted
labels. Target admission scans A current-floor actors, then a visibility ray and
existing body-origin scan for a matching coordinate; worst case O(A + B*R) with
B<=2 and bounded map ray R, plus core's O(A) living-target check. No hidden actor
values enter UI output before visibility admission. Adjacent application renders
at most one target cell, never a projectile trace. Inventory provenance/name
formatting is O(1) beyond string lengths per displayed item. Pack snapshots retain
the existing O(I) inventory equality cost and refresh behavior.

Native screenshots of both completed Witch-species pages at1280×800 and1920×1080
are under `monster-hooks/verification/pages/` in the working delivery tree. The
1280×800 captures were visually reviewed for unclipped text/art and absence of
placeholder/matte. Screenshot generation used the live bestiary renderer, not an
image composite.


### Completed art/page integration checkpoint

All three new plates (Night Crawler, Skin Walker with approved localized blur,
and Saidribee Queen) are accepted and integrated unchanged. The runtime contains
84 RGBA plates and 82 bestiary pages. Hash/alpha verification covers all84;
27 bestiary tests pass (one optional timing benchmark ignored), including all
page layouts and preserved full text. The Queen page is also captured at1280×800
and1920×1080. Original81 source masters, original81 runtime RGBA references, and
all79 original Natural History strings are unchanged; evidence is saved in
`verification/original-art-prose-preservation-v33.json` in the delivery tree.


Final-stat captures use the `*-final-1280x800.png` / `*-final-1920x1080.png`
filenames. Pixel comparison against the preceding reviewed captures confirms
that only the intended Accuracy/Dodge number regions changed at both sizes.
Post-overlay focused tests pass:11 core Witch/Skin/Queen metadata cases and7
observer/projection cases. The earlier targeted damage percentages above are
not final tuning results; see the Accuracy/Dodge subsystem's final measurements
and cohort provenance for that later change.

## Revision33 combat-message identity correction

`actor_presentation::apparent_kind` is now the single read-only selector shared
by desktop creature projection and actor-aware core message producers. Ranged
hit/miss narration previously used a visible actor's true name even when its
map appearance was disguised; generic damage, death, status, movement and
name-only actor-event labels now use the apparent name at the message boundary.
Each substitution receives the actual actor reference. No completed log string
is globally searched/replaced, so two Skin Walkers can retain different names.
Witch birth announcements remain generic rather than disclosing offspring.

Visibility admission remains in the original callers. Death reasons capture
source appearance before a lethal packet removes a living observing body;
`RunDeath.source_name` remains true provenance. Actor identity, stats, AI,
conditions, RNG, hearts/rewards, event coordinates and simulation target IDs
are unchanged. Revisions29–32 return their original names and messages exactly.
Chronicle entries describe the appearance when generated, not a retrospective
reinterpretation at a later hallucination tick or observer position.

The selector costs O(A + B*R + L) per invocation: actor/body-origin scan A,
B<=2 observer bodies, line-of-sight ray R, and seed bytes L; cold catalog parsing
retains its existing cost, then bounded disguise lookup is expected O(1).
Existing visibility helper costs are additional at source-summary admission.
For M named messages per normal turn this adds O(M*(A+B*R+L)) worst-case work,
not an unqualified O(M). Visible name results borrow actor/catalog strings;
only a potentially lethal packet captures an owned label (O(label bytes)) for
later narration. No new persistent cache, save field or frame-driven mutation
is introduced. Existing native text-render/cache costs are unchanged.

Focused regressions cover actual far/near Bow hit and miss commands, two
independently disguised actors, NPC attacker labels, environmental damage and
kill text, pre-lethal close/far physical/poison death reasons with true source
provenance, Half Here, hallucination, save/replay, Revert/See The Future, and
unchanged legacy labels. A separate actor-event test verifies labels differ by
actor while event geometry and glyphs remain unchanged.
