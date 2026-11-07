# Revision 51 two-press autoexplore

X in unobstructed gameplay previews a route; a second deliberate X starts the
existing automatic travel controller. Holding X cannot confirm or cancel it.
Item/ability targeting and existing modal controls take precedence. The preview
stays highlighted when the mouse moves. Other keys, a mouse press/scroll, or loss
of window focus cancel it; Escape only cancels the exploration, without opening
the main menu. There is no new status text, tooltip, or help prompt.

The destination is the nearest reachable **known frontier** bordering an unseen
square, under the existing route safety/distance ordering. Unknown terrain is
never read to choose walkability, and the path never claims an unknown square is
safe. It does not blindly step into black space. A frontier can reveal more map
on arrival; in total darkness it may reveal nothing. In that case, subsequent
X requests skip already visited frontier positions while the unseen-cell mask
is unchanged, eventually returning an empty preview. Fresh knowledge or another
input resets that temporary history. Fully explored, inaccessible and missing
legacy-memory maps safely return no route.

Exploration excludes observed occupied cells, stairs, pits and stochastic
crossings from route selection. It retains deterministic portal arrivals and
normal/haste movement phases. Actual commands still use normal movement and
hazard confirmation; exploration never approves a hazard itself. Observed new
threats, harm, incapacitation, blockers, changed depth and unexpected arrival
stop travel. It also refuses to turn an exploration step into a known attack.
Automatic exploration interruptions do not add notice text. Ordinary click
travel preserves its existing behavior and notices.

This is detached desktop state only: no save/replay fields, RNG draws, gameplay
turns on preview, or simulation mutations are added. After confirmation, each
actual move passes through the existing recorded dispatch path, including
normal/haste/off-screen processing. Load, Revert, Future and modal navigation
cancel the detached selection through their existing input events.

## Complexity and limits

Let N be map cells (normal native floor 79×29; admitted pathfinding bound 3,600),
S=4N movement-phase states, A floor actors, K known portal/non-Euclidean entries,
and L output steps (at most S). One preview reuses terrain visibility and the
existing Knowledge projection, including their existing optical/lighting,
actor visibility, ability/catalog and topology lookup costs. These are not
claimed O(N): see the visibility/pathfinding subsystem contracts. Added unseen
mask comparison and frontier enumeration are O(N); eight neighbors per cell
are fixed. Temporary visited/frontier/mask/path storage is O(N+L).

One multi-goal search replaces running a path search for every frontier. With
bounded degree eight and constant-time phase transitions, it is O(S) time/space
for the unweighted queue and O(S log S) time/O(S) space with existing wet/hazard
priority weights. Hash-set membership is expected O(1), worst O(N), so adversarial
hash behavior can make these O(NS) and O(NS log S) respectively; this also applies
to the existing route implementation. Stable direction order breaks ties.
Topology and observation setup costs above are additional. No new per-frame
search runs. Preview confirmation transfers the prepared controller without
copying its path. Preview painting uses the same O(L) coordinate projection,
path-tint raster and existing retained/tile damage costs as click travel.

Actual automatic steps retain existing movement dispatch, hazard checks,
visibility/threat observation and replanning costs. Exploration adds one
pre-step observed-threat snapshot (A filtered visibility checks and O(A) data)
and blocker refusal; it does not treat visibility as constant-time. Current
threat comparison uses a hash map, expected O(A), worst O(A²). The retained
no-progress frontier set is bounded by N and resets on new knowledge/input.
These are analytical bounds, not measured timings.

## Verification

Deterministic tests cover preview-only first X, second-X transfer, cancellation,
known-frontier shortest route, hidden-wall/hazard/actor invariance, observed
occupancy exclusion, complete/inaccessible/legacy maps, repeated no-progress
exploration without oscillation or blind stepping, new-knowledge reset,
newly observed threat interruption and preserved hazard confirmation. Existing
route fixtures continue to cover stable ties, phase/displacement, portals,
random crossings and harm/arrival cancellation. Input tests cover cancellation
versus hover and X. The parent coordinates native compilation and full workspace
execution; tests are authored, not marked passed by this document. Native
Windows input and visual highlight verification remains a separate check.
