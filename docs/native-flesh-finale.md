# Native Flesh Administrator and escape finale (revision 31)

## Implemented scope

New revision-31 floor 40 has exactly one The Flesh Administrator. She is a plain
female fighter, identified by name and `A` map glyph, with no new spells, phases,
bestiary pages or raster art. A separate deterministic post-population placement
pass selects the farthest reachable safe unoccupied ground cell from the arrival
stair (stable cell-index tie break), avoiding arrival/stairs, loot and fixtures.
Ordinary encounters, their exact heart budget and their RNG draws are unchanged.
The boss is heartless and does not revive. Revision 29/30 generation and old
bottomless-pit victories remain unchanged. New revision-31 floor 40 replaces the
old bottomless-pit endpoint with ground, so it no longer bypasses the finale.

Provisional boss values are centralized in `finale::administrator`: 1450 HP,
Offense 550, Defense 225, Accuracy 115, Dodge 55, DR 60. Existing health sync is
100 + 6 × Defense, and ordinary adjacent attacks supply their normal base damage,
variance, hit calculation and scheduling. No per-hit or action-order override is
added. The chosen target is a serious ~70–90% whole-fight resource cost, not an
additional designer-authorized percentage-per-hit rule.

Native actual-dispatch testing by the balance worker on 128 random joint
full-prior-heart floor-40 reference profiles gave 82.42% mean whole-fight HP loss,
39/128 deaths (30.47%), and zero censors. A separate first-64 native cohort gave
82.03% loss and 29.69% deaths. The independent translated 100,000-trial model gave
81.86% mean loss and 29.53% deaths. These are stationary default-weapon trades
without active player powers, gear decisions, healing or tactical escape, not a
guarantee of survival or a finished balance judgment for real builds. The native
harness stops at the actual boss-death boundary; it does not test the ascent.

## Exact collapse and outcome rules

Actual death is intercepted only after existing rescue/death resolution. The
boss is removed instead of becoming a revivable corpse or releasing a heart.
The immediate chasm starts at her actual death coordinate, even if she moved or
died away from her original placement. A saved run-level defeat/collapse record
prevents a second trigger. Ordinary enemy deaths never start it.

The collapse consumes one eight-neighbor (Chebyshev) adjacency layer every two
normal world turns. The immediate death cell is layer zero; the first expansion
is at death tick + 2. Haste-only actions do not advance this clock, while an
actual extra normal turn caused by slowness does. Each pulse reads only its old
frontier, so new neighbors do not expand again during that pulse. Walls and
physical fixtures do not block this collapse. Gas already in open air retains
its ordinary behavior; solid layers, physical lights, fires, grates, doors,
bulwarks and reversible-chasm restorations are removed at consumed cells.

Cross-floor behavior is a provisional implementation assumption communicated to
the designer, not a designer-chosen rule: when the wave reaches the up stair on
floor N, it immediately seeds the matching down stair on N−1. Stair identity is
captured before conversion. A newly seeded floor does not gain an extra spatial
layer that pulse. Collapse advances on reached inactive floors too, and never
starts every floor at once. Existing seeded floor generation is the only allowed
way to obtain a missing upper floor. Normal play has already generated all upper
floors on the descent. No random wave placement or simulation RNG is used.

Collapse-created gaps are permanently bottomless. Loose objects, corpses and
non-flying actors fall out of play, without bonus death rewards or hearts.
Ordinary chasms retain their existing next-floor behavior. Existing flight from
conditions, abilities, equipment and Phase Through is preserved. Hovering is not
a fall; an explicit dive is. Flight loss and forced fall cohorts use the same
collapse provenance, including inactive actors. A falling Half Here endpoint is
dismissed rather than declaring that its still-supported owner fell.

An actual player fall records the exact reason and chronicle message:
`Fell while escaping the collapse of the flesh.` It is terminal, including on
floor 40. Returning to the existing intact floor-1 blue Ω entrance after the
boss's defeat sets `won` and records the exact ending `Collapsed the flesh!`.
Entering the door before defeat does not win. The door's existing style and
metadata are reused. A floating player who waits until the escape stairs or door
are consumed is not silently killed and cannot win at a nonexistent door. The
chronicle warns when the active floor's up stair or entrance is consumed. Such a
lost route is an intentionally disclosed consequence of preserving flight, not
a new fallback ending or immunity. Escape success is not guaranteed on every
route/build; walls do not delay the advancing wave, so corridor length matters.

## Persistence and compatibility

The optional `Run.collapse` is shared by `Arc` with temporal snapshots. It stores
boss identity, absolute normal-turn pulse clock, escaped flag, and one mask,
frontier and saved up-stair coordinate per reached floor. Save admission rejects
invalid clocks, duplicate floors/frontier entries, out-of-bounds cells and
provenance whose terrain is no longer a chasm. Snapshot cloning, inverse Revert
patches, See the Future anchors and offscreen proxy merges all retain the state.
Rewinding before death restores the living boss and removes collapse; restoring
a later state restores its exact frontier/parity. Replay uses the same seed and
command path. Playtester origin remains synthetic/unranked. Endings persist in
detached profile archives without changing old summary shapes, as described in
[native-finale-presentation.md](native-finale-presentation.md).

## Complexity and bounds

Let F≤40 be generated floors, V≤3600 cells per floor (normally 2291), A the actors
on a processed floor, I its loose objects, N total world actors, Q the current
frontier size, R the number of reached floors, and L the removed actors in a
pulse. One-time placement costs O(V + A + I + V·G) time where G is the grate-list
length used by allowed-cell checks; the fixed bounded-degree path search is
O(V), not one path search per candidate. Extra placement memory is O(V+A+I).
Door lookups are expected O(1) sparse-map operations. Fixture/hazard layer count
is fixed. No gameplay or population RNG is consumed.

Each normal turn checks the pulse clock and active-cell provenance in O(R).
A pulse visits at most eight neighbors per old frontier cell and marks each cell
once: total frontier work over the entire collapse is O(FV). The implementation
also detaches the shared collapse block and touched terrain blocks when required
by a live temporal snapshot, scans fixture lists and actor/object cohorts, and
copies each reached mask for object filtering. Per pulse this is
O(sum over reached floors of (Q + V + A + I) + L·N + F²), plus the fixed-cost
flight-ability/catalog and inventory checks per actor. L·N explicitly accounts
for existing maintained-link cleanup per removed actor. Floor lookup and the
bounded stair scan account for F². Sparse sets have expected constant-time
membership. Extra pulse scratch/storage is O(FV + N + I), including COW masks
and terrain plus the existing link-cleanup lookup scratch. Historical snapshots
share unchanged blocks but changed pulses may retain up to the existing five
completed frames plus pending/future anchors; serialization costs their encoded
size rather than O(1). All finite map cells eventually leave the frontier.

Fall/placement interception performs O(R) provenance lookup and adds no new
pathfinding. Terminal/profile rendering costs are in the linked presentation
ledger, including native glyph-cache misses and framebuffer work. Floor
generation, offscreen environment simulation, visibility refresh and the
existing actor AI keep their own documented transitive costs; this feature
makes no linear-time claim about a full world turn. Missing-floor generation
adds its existing seeded generator/population cost. No measured end-to-end
frame timing is claimed. To profile, use real floor-40 seeds, trigger collapse,
then measure ordinary turns with all reached floors and live temporal history.

## Verification scope

`tests/flesh_finale.rs` covers unique/safe/budget-neutral spawning, actual moved
boss death and one trigger, no RNG/corpse/heart, pulse parity and simultaneous
layers through walls, immediate stair seeding, continued offscreen spread,
active/hidden fall cohorts, flight/dive distinction, ordinary fall compatibility,
fixture/object/corpse removal, Half Here endpoint dismissal, only-after-defeat
entrance victory, actual haste scheduling, save/load, fixture replay/restart,
Revert and See the Future restoring both pre-death and in-collapse state,
revision29/30 old endpoints and floor-40 unranked playtester admission. Generated
spawn tests use three full forty-floor seeds; this is deterministic sampling,
not exhaustive generation or tactical winability proof. Exact terminal strings
also have native software-painter pixel tests and inspected PNG review frames.
Full-workspace, Windows runtime and integrated display checks are recorded by
the final independent validation owner rather than implied here.
