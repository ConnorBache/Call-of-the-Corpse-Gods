# Revision 50 desktop integration

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

Per-run metadata now comes from the versioned core catalog. Revision 50 does not apply legacy description overrides. Pack shows five sections (trinkets, amulets, rings, potions, hearts), automatic carried-jewelry activity, direct bow/amulet Apply, individual category counts/budgets, and all retained plus additional Tinker grants. Legacy runs retain their equipment actions and presentation.

Configured ability reserves appear as “Reserve current/capacity” alongside the existing cost line and in ability/item-grant details. Paid powers without an authored reserve show “Reserve not configured”; no capacity is inferred from activation cost. Affordability and Tinker admission use the core reserve-first payment plan, including atomic item/ability-first split payment. Details explain exhausted-reserve fallback and level-up refill of ability energy only. Existing layout and all pre-50 text remain unchanged. Creature details show that creature's configured reserves, never the player's.

Implant Thought selects a visible enemy first and a visible adjacent creature second, including the player. Invalid second selections retain targeting without payment. Escape cancels; selecting a new power resets the first selection. Fated Duel requires a visible adjacent enemy. Waking Nightmares targets a visible unoccupied square. Relentless Pursuit targets a visible enemy and displays the core route's contiguous visible prefix. Supported targeted abilities use revision-aware flags and the per-run runtime support registry. Missing runtime support disables command binding rather than invoking legacy behavior.

While targeting Cogworks, keys 1/2/3 select its serialized mode without spending an action. The mode and energy cost appear in the notice and ability row; sight previews use Kative's orange. Mode 2 creates the five-cell wall with one shared200-HP pool. Pit placement and free removal use the core preview and command APIs. Clay Army selects an adjacent empty destination and previews the departure tile where the wall will be placed. It uses the core run-level movement admission and existing ability-reserve payment rules. Its details explain the live current-player attack without abilities. Persistent Clay walls retain their ordinary wall glyph with the label “Clay Soldier (unbreakable wall)” and Kative's orange.

Placed Cogworks pits and stored walls also retain Kative's orange through a detached terrain Cell marker and animated terrain reconstruction. Only currently observed matching terrain receives live placement ink. Remembered or unknown terrain is never recolored from current hidden modifications: existing remembered terrain remains intact, without inventing saved Cogworks-color provenance. Pocket overlays, glyphs and other authored map treatments remain unchanged. The retained renderer compares complete Cells; the tile renderer explicitly compares the marker as well as its ink and overlay.

The existing enemy column, corpse marker and blood-dot treatment remain intact.
The later visual follow-up reduces modal sheets from 88% to 84%, preserving
the R46 composition, content, shared input/damage transform and appearance
preferences. Chronicle, gameplay typography, Pack bounds and nonmodal hover
scale are unchanged. See [visual follow-up](revision50-visual-follow-up.md).

## Complexity and verification

Metadata selection scans A catalog entries (A=72), O(A + text bytes) per ability projection; first catalog use parses fixed authored text. Category summary is O(B) for owned blessings, constant scratch plus output string, once per Pack refresh. Additional grant descriptions cost O(G*A + text bytes) per item, G stored grants. Existing Pack grouping remains O(I²), including full grant-vector comparisons and text output. Target checks scan current-floor actors and perform existing visibility/plane queries; Implant victim admission is O(Ac*V) worst case for Ac actors and V the cost of a visibility query, with constant scratch. Candidate enemy preview currently also performs actor lookup per candidate, yielding O(Ac² + Ac*V) worst case. Cogworks invokes bounded nine-cell core geometry plus occupant visibility and the terrain-visibility mask; removal scans stored terrain modifications. Pursuit uses the core route planner plus one visibility mask and O(route length) filtering; its grid/actor/portal bounds are documented in revision50_pursuit.rs. These queries run on observation or pointer invalidation, never during raster painting. Color changes participate in both damage paths. These are source estimates, not timings.

Added unit tests for revisioned metadata and costs, equipment action filtering, category totals and multiple-grant descriptions. Rust compilation, SDL interaction and visual QA have not been run in this environment. Run tools/cargo.sh test --workspace and manually verify repeated targeting, cancel/reselection, modal scrolling and old replay rendering before enabling production revision 50.

Placed-color projection adds O(V + K + C) time and O(V) temporary bits once per world observation, for V observed grid cells, K total stored modification footprint cells and C projected terrain cells. Animated reconstruction restores only detached marker, ink, overlay and label data, O(label bytes) per marked cell. Added regressions cover live ink, reconstruction, hidden/remembered invariance and legacy rejection. These Rust tests remain unrun without the toolchain.

Reserve lookup/payment projection is expected O(ID bytes) per ability with O(output text) allocation; IndexMap collision worst case scans B stored reserves. Pack refresh stamps copy B reserve records and ID bytes so reserve-only changes invalidate tooltips. Added tests cover actual configured capacity, own-reserve affordability with zero player energy, exhausted fallback, missing/partial reserve messages, read-only observation and unchanged legacy output. No Rust tests were run: cargo remains unavailable, and no installation was attempted.

## Later named-ring and amulet UI pass

Pack entries use canonical names, including Vampirism and Ice Immunity, and the
current authored effect descriptions. Light's additive-copy exception is shown
instead of the ordinary nonstacking note. Amulet Apply enters existing UseAt
selection without introducing a replay command; Escape/Cancel remains free.
Known-only effect previews replace the old adjacent-creature-only preview for
amulets, while Borge Shards retain their prior targeting rules. Tooltips show
current item energy out of 50, current Appropriate-adjusted cost and no cooldown.
Item energy and normal tick already participate in the exact Pack Stamp.

Discord, Roots and Acid corrosion have named timed status meters. Their internal
source/anchor/penalty fields are never exposed as status names. These additions
are revision-gated and leave the preserved layout, draft colors and legacy
item/status behavior unchanged. Final native/UI verification is pending the
coordinated final-source run; focused tests are authored alongside the changes.

## Item knowledge and concise copy

R50 row hover now shows canonical item names and effects before selection, with
approved short jewelry sentences followed by ranges, durations, energy and
exceptions. Carried jewelry is labeled active and offers no Wear/Take off control. Potion identity never
requires use; the same read-only names/descriptions reach admitted ground items,
map labels and new item-action logs. Hidden live items remain behind existing
observation gates. See [item knowledge](revision50-item-knowledge.md) for exact
scope, legacy preservation, complexity and pending verification. The unrelated
layout/draft and all release admission guards are retained.
