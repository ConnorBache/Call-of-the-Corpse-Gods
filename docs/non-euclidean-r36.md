# Revision 36: Non-Euclidean pockets

The designer specified contiguous pockets from floor 20, cellular-automaton shape
selection, sequential attachment assignment, traversable space, and **no chained
vision through these attachments**. Two sources share one destination; the
reverse traversal chooses randomly between those sources. Destinations must be
unassigned squares, not arbitrary previously assigned squares.

## Literal assignment and bounded safety rejection

For a new component, the first source is uniformly sampled from its unassigned
cells. Subsequently, the first source is the randomly selected, still-unassigned
legal eight-way neighbor of the previous destination. A draw proposes one source 70% of
the time and two sources 30% of the time. The second source, when proposed, is a
distinct uniformly sampled remaining cell; no additional physical-adjacency rule
for the two sources was specified or added.

Sources are removed **before** computing destination eligibility. If only two
cells remain at the start of an assignment, select one source. If a proposed
second source would leave no unassigned square with an unassigned neighbor, undo
that second-source proposal and use one source. Uniformly choose the destination
among unassigned squares with an unassigned legal eight-way neighbor. Only when there
is exactly one unassigned square left may that square be the destination without
such a neighbor. Remove the destination and uniformly select its unassigned
neighbor as the next forced source. Assignment adjacency matches live eight-way
movement, including the existing wall-corner rule; CA components remain
cardinally connected and the safety proof remains cardinal and conservative.

This literal process can strand a singleton, or leave no legal continuation.
Such an attempt is discarded in its entirety. It is **not** repaired by adding a
self-link, merging groups, redirecting an edge, or linking an assigned square.
The 70/30 ratio describes proposal draws, not a guaranteed final ratio after the
designer's fallback rule and safety rejection.

Accepted groups are `A ↔ C` or `A/B → C`, with `C → A/B` using a fair 50/50
simulation-RNG choice. Each cell belongs to exactly one group. The saved order
retains the generation trace; it is not rerolled on observation or load.

## Initial numerical defaults

These are conservative implementation defaults, not designer-authored scaling:

- Request one pocket on each generated floor 20–40.
- Use a local 11×11 mask, initial live probability 0.55, and three Moore-neighbor
  CA rounds: a dead cell is born with at least five live neighbors; a live cell
  survives with at least four. Ineligible cells remain dead.
- Select a complete cardinally contiguous component containing 6–48 cells.
- Try at most 48 shapes and 64 complete assignments per shape. If none is safe,
  omit the pocket; do not rewrite terrain or force a broken link.
- Protect stairs, special fixtures, generation-protected areas/connectors,
  portal endpoints, and initially occupied/loot cells. Eligible terrain is floor,
  grass or shallow water. Generation uses a separate seed/floor stream and does
  not change the ordinary actor/heart/loot budget or simulation RNG.

The safety check constructs the **actual directed walking graph**, including
both possible stochastic arrivals and existing portals. Every originally
reachable ordinary walking cell must remain reachable from the upstairs and
able to return to it. A second directed proof excludes all portal endpoints, so an accepted pocket
cannot make previously optional portal traversal necessary. Thus either
randomized outcome has an escape route; a
physically contiguous CA component alone is not accepted as proof. The proof
uses cardinal walking, conservatively within the live eight-direction movement
rules. It does not promise immunity to later terrain changes or combat hazards.

## Movement, occupants and physical effects

Entering an attached square performs one transition. Arrival does not immediately
bounce again; the next movement begins from that actual arrival. Links never
change the actor's waking/dream plane. Half Here retains its existing creation
plane. Occupants and ground items keep one authoritative identity across linked
representations, rather than duplicating actors or inventory entries.

The reverse choice uses serialized gameplay RNG only when a transition actually
commits. A pure query, hover, preview, save, replay display, or rejected movement
does not consume a new lottery draw. All possible movement outcomes must be
admissible; a blocked branch is not silently removed to bias the fair lottery.
Same-plane occupied linked space blocks or uses ordinary attack/ally interaction
rules rather than telefragging. AI pathing retains possible exits and replans
from the actual arrival, rather than selecting the favorable random result.

Physical shots and thrown objects follow attached space without creating a new
viewpoint. Direction and remaining range continue across a transition, with
cycle guards. Pure trajectory previews stop at uncertain branches and disclose
possible exits rather than falsely drawing one chosen result. Actual actions
sample only forks physically reached by the committed projectile. An actor on a
linked square may also launch from its attached representation, as with portals.
A deterministic alternate origin requires no draw. If the alternate origin has
two possible sources, the plan retains that unresolved origin fork; the actual
launch draws once and translates the planned direction/range to the chosen
origin. A direct physical-origin shot does not draw merely because the shooter
stands in a pocket. NPC shot admission uses a bounded pure possible-reach probe under existing
local sight and range rules. It examines at most 256 explicit branch prefixes
against the same fixed ray plan; if no target witness is established within that
AI search budget, the NPC declines the shot. The budget does not alter actual
projectile physics. It does not inspect future RNG or select the
favorable actual branch. A launched action still samples the committed trace;
a random miss consumes its ordinary resources, cooldown and recovery.
Directional fire cones retain a conservative heading check: an uncertain central
heading may be declined. Once admitted, each deduplicated emitted cone ray uses
committed physical branching and ignites its actual landing; autonomous fire
propagation is still unchanged.

**Vision remains local to the physical observer for these pockets.** They do
not project sight, reveal an exit room, shorten optical distance, or chain new
viewpoints. Revision-35 portals retain their separate authored optical behavior,
italic ASCII `0`, pair colors, and existing occupancy/projectile behavior.
Autonomous gas, fire and liquid propagation is not newly transported.

## Persistence and presentation

New descents and DEBUG floor starts use revision 36. Runs/replays from revisions
1–35 retain their historical semantics; unsupported future revisions are rejected.
Pocket data lives in ordinary floor state and therefore participates in save/load,
replay, Revert and See the Future. Legacy floors without pocket metadata are not
retrofitted on load. Structural admission rejects overlapping/duplicate/out-of-
range members, null metadata, disconnected components, invalid group partitions,
and portal overlap. Later altered terrain is not mistaken for corrupt metadata.

Pocket terrain has a cyan inset without replacing its original glyph. Hollow cyan
corner marks show possible known exits, never a selected random result. Auto-travel
may traverse deterministic known attachments; a stochastic entry is allowed only
as the explicitly selected final step, after which a new route must start from
the actual landing.

Only observed/remembered information can support a preview; unknown exits remain
unknown. The UI consumes detached data and cannot consume gameplay RNG. See the
revision-36 verification receipt for executed coverage and renderer evidence;
this rules document itself is not a claim that a check passed.

## Epidite display refinement

The Epidite's on-map timer is shown only for single-digit countdowns (9 through
1). Its observed description says `X turns until judgement`, using the existing
timer for X. The 30-turn cadence, pulse damage, and core countdown are unchanged.
The rollover to 30 hides the map subscript again. Unseen towers do not disclose
a fresh countdown through a hover description.
