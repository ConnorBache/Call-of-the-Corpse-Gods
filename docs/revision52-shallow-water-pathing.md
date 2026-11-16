# Revision 52 shallow-water pathing

## Behavior

Revision 52 player route planning gives shallow water ordinary movement cost.
Hover previews, distant click travel, automatic replanning and two-press
exploration all use the same detached Knowledge projection. A shorter safe route
through shallow water is selected instead of a dry detour. Fire/gas safety
priority, command count, diagonal tie-breaking, movement phases, observable
occupants and transport rules remain unchanged.

Deep water remains excluded as a route entry, deliberate route endpoint and
known deterministic transport arrival, including with Swimming, flight or Drown.
The existing ichor-covered-water exception is preserved because ichor suppresses
water contact. A player already in deep water can plan an exit onto nearby
shallow water or dry ground, but the planner does not choose another deep step.
Deliberate adjacent clicks and keyboard movement still use the ordinary movement
rules, so manual swimming is not disabled. Swimming-ring powers, currents, isolated-island generation and escape validation
remain owned by the core scheduler; this desktop change does not modify them.

Observed terrain is used only where the existing visibility projection permits
it. Otherwise planning uses remembered terrain and remembered ichor. An unknown
square or missing legacy tile memory remains unroutable. Hidden live deep-water,
shallow-water, wall, current or ichor changes do not rewrite a remembered route.
A fresh automatic replan rejects a newly known deep-water bridge. As before,
remembered routes are not a guarantee that unobserved terrain has stayed safe.
The pre-52 wet-cost preference and frozen differential fixtures are unchanged.
The desktop adds no save fields, RNG draws, new commands or simulation mutations.
Its R52 projection reads the core scheduler's optional fastActionPhase field.

## Fast and Haste integration

R52 routes distinguish ready-Haste, ready-Normal, pending-Fast-Haste and
pending-Fast-Normal. Each stage is paired with the existing cameFree flag.
An initial successful Fast move saves the owning complete-action stage without
a world tick. Completing pending-Fast-Haste returns to ready-Normal without a
tick; completing pending-Fast-Normal advances world time and returns to
ready-Haste. A normal complete action may still use its independent I Came For
You credit. Pending stages are preserved even after the originating Haste/Fast
source is lost, matching the scheduler's already-earned continuation.

Swimming is sampled from each departure cell in the permitted terrain/ichor
projection. Entering shallows does not retroactively change the action already
under way; leaving water retains any already-earned bonus. Non-water powers and
status counters remain observation snapshots, with a fresh plan after every
executed automatic command. Future enemy actions, expiring statuses and newly
revealed terrain are not simulated by the pathfinder.

Move and Wait hazard forecasts share the pure four-stage timing helper. An
ordinary Fast first move and either Haste stage do not forecast a world/current
tick. Wait never earns a new Fast continuation, but can complete a saved one.
A true-Haste Wait therefore does not warn about tick-only exposure/current
drift. Movement contact warnings remain independent of that timing decision.
Legacy movement-only Haste and legacy Wait warnings are unchanged.

Before a new Haste-stage action, known adjacency Chill can suppress both speed
sources. A detached cold mask includes only visible actors whose true identity
is not disguised or hallucinated. It respects Ice Immunity, Oblivious and the
planes of both owned Half Here endpoints. Unknown actors and hidden identities
never change a route or warning. Known transport endpoints form zero-distance
components, so the mask also handles adjacency through known portals/spatial
links without reading hidden terrain. Other-plane owned endpoints use their
own floor's remembered transport links. A pending Fast continuation does not
re-run the pre-action Chill check.

The warning projection counts zero, one or two world advances. Known new Chill
and existing Slowed produce the scheduler's two normal advances, including a
warning for a second observed current landing. Each current endpoint must pass
the existing visibility/occupancy gate. A stranded player advances once through
the core's earlier habitat branch and does not suppress Wait exposure warnings.
Private unknown effects, changing actors and terrain are outside the guaranteed
preview contract. Ordinary gameplay still performs all actual checks.

## Complexity

Let V be floor cells (at most 3,600), S=8V R52 movement-phase states (4V for
legacy runs), A observed actors, K transport metadata and L output route steps.
The phase graph has at most 28,800 states. Stage transitions and per-departure
Swimming checks are O(1); the existing per-cell wet mask remains O(V) time and
memory. Each Knowledge construction additionally checks carried Swimming-ring
eligibility once, O(I+C) for inventory I and actor conditions C, and reuses the
existing non-water Haste observation costs, including any actor/detection checks. R52 sets that mask false, so shallow water alone no longer
selects the weighted-search branch. Route transition checks remain O(1) expected
with the existing indexed occupancy/topology sets. With eight neighboring edges
per state, the queue search is O(S) and hazard-weighted heap search is O(S log S),
with O(S) graph storage and bounded-degree heap entries. Existing hash-set
worst-case behavior can add an O(V) factor to transition membership checks.

Visibility, hazard-context construction, actor queries and transport projection
costs remain additional, as documented in travel-ui-complexity.md and
revision51-autoexplore.md. Each successful automatic step retains its existing
observation/replanning and normal engine dispatch costs. Preview rendering still
receives only detached steps; no new frame-time search or native rendering work
is added. This is asymptotic analysis, not a performance measurement.

For the cold mask, B<=2 owned body endpoints and Y permitted cold sources are
bounded by actors A. Let Q include a source's visibility and detached identity
projection costs (actor/body scans, rays, template data and copied actor state),
and O include the owner/Oblivious visibility query cost. Source admission costs
O(A*Q) in the worst case, plus the existing actor-ability scans. Union-find over
known transport links K and V cells costs O((V+K)*alpha(V)); one fixed eight-
neighbor pass computes component adjacency in O(V+Y). At most B+1 component maps
are built, including the other plane's remembered map when needed. Ring immunity
costs O(B*(I+C)); source/body admission additionally costs O(B*Y*O). Extra scratch
and retained masks are O(V+A+K), with B a fixed bound. No per-route-edge path
search or visibility query is added: each transition reads the mask in O(1).

A hazard timing query can reuse the full Knowledge construction when an
unprojected visible cold source exists, including terrain visibility and all
observation costs above; this is not an O(1) whole-query claim. The pure stage
transition itself is O(1). Each forecast checks at most two current steps, each
with existing visibility, hazard and actor-occupancy costs. Warning branches
may invoke the timing query a bounded number of times (including the eight
possible confused directions and at most two spatial exits). No caching is
claimed. Representative profiling of this cold-source boundary remains pending.

## Verification

The dedicated revision52_shallow_water_pathing_tests suite covers visible and
remembered shallow routes, normal/haste phases, preserved pre-52 preferences,
deep barriers/endpoints with Swimming/flight/Drown, a shallow bridge through a
deep barrier, hidden live terrain/current/ichor invariance, unknown and missing
memory rejection, independent fire avoidance, hover/click path agreement,
real automatic engine dispatch with and without Swimming haste, manual deep
entry, replanning after bridge deepening, autoexplore privacy and portal arrivals.

Focused native execution passed against the combined compile18 binary, as
recorded below. Full-workspace receipts remain coordinated by the parent build.
This document does not claim native-window input testing or a full-workspace pass.

### Shallow-routing baseline receipts

The combined compile16 binary passed 29 of the initial 30 travel tests. The
remaining new autoexplore fixture incorrectly used Perception 300 while assuming
-100 light hid its frontier; R51/R52 correctly applies the literal perception
formula instead. The test now uses Perception 0 and asserts the relevant cells
are actually hidden. No production correction was needed. Additional assertions
cover leaving deep water onto shallows and the preserved ichor-covered-water
exception, including deterministic transport. The final compile18 binary passed
all ten R52 tests and all 21 existing travel tests (31 total, zero failures).

The exact compile18 binary also passed 43 related portal, non-Euclidean,
Dreamwalking and water-render regressions. One opt-in native SDL GPU case was
not run. Final receipts: verification/r52-shallow-water-pathing/travel-focused-18.log,
verification/r52-shallow-water-pathing/related-focused-18.log and receipt-18.json
in that directory. The original compile16 fixture failure and related passing
run are retained separately rather than overwritten. These receipts predate the later Fast/Haste integration described above.

### Fast/Haste integration verification

The separate revision52_action_pathing_tests suite checks every saved stage
encoding against real engine Move/Wait dispatch, conditional Swimming entry and
exit across complete routes, Fast substages, remembered-water/ichor privacy,
autotravel and autoexplore replanning, manual deep-water and portal/current
warning timing, lost traits and Slowed during an earned continuation, cameFree,
contact warnings and legacy Wait behavior. Additional checks cover visible pre-action Chill, both speeds absent, second
current landings, hidden sources, Ice Immunity, Oblivious, mixed-plane Half Here,
known versus hidden portal mates, stranded Wait, and paired hallucinated
identities with different raw cold traits. All 14 integration tests passed in
compile20, together with the ten shallow-route tests and 21 existing travel
tests (45 travel tests). All 15 hazard, seven portal, 18 non-Euclidean and 12
Dreamwalking tests also passed: 97 passing checks in this focused scope.

Compile19 found five test-only calls to a private core mutation helper. They
were replaced with the existing public actor_mut API without changing core
visibility or runtime behavior. Its compilation log remains preserved.

The wider status filter passed 13 cases and found one separate legacy hover-copy
regression: flight-vampire-0 received the new R52 Haste description instead of
"Haste · ongoing." That failure was reported to the status-text owner and is
preserved in related-focused-20.log. It does not certify a full-workspace pass.
Phase/hazard runtime source remained frozen through the passing compile20 run.
Receipts are in verification/r52-action-pathing/, including the exact binary
hash and the separate travel, related and transport logs.
