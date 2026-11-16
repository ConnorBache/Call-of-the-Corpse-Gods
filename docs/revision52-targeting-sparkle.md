# Revision 52 reachable targeting and lasting Sparkle

## Designer authority and compatibility

On 2026-11-07 the designer clarified that selecting past a blocking creature
selects the reachable line, with Iceblast detonating on that creature. Sparkle
must stay lit until something extinguishes it; Ichor-covered water does not.
These rules supersede the pending decisions in `revision51-aiming-paths.md` and
`revision51-ability-restorations.md` for revision 52 only. Older saves/replays keep
their existing collision, remote Iceblast, Blink rejection and wet Sparkle rules.

## Executed rules

- Iceblast follows the finite physical ray toward its selected visible, open
  square. Its first creature intercepts the blast. Terrain stops delivery at the
  last reachable square. The existing 3-by-3 cold/slow/ice footprint is centered
  there, retaining its existing wall/corner exclusions. Actor membership uses
  physical portal aliases, so a blocker at a distant mate receives exactly one
  cold/slow packet while the physical ice footprint stays local. Self-targeting remains
  legal. Its existing cost and unlimited targeting distance are unchanged.
- Blink follows its physical ray and lands at the last accessible, unoccupied
  square on that line. It never overlaps an interceptor. Both selected and
  resolved landing squares retain the ten-tile Euclidean limit. Known blockers
  with no forward landing reject before payment. A hidden blocker cannot change
  admission: it can stop the paid cast at its origin without rerunning arrival
  effects. Walls, closed doors, trees and fixed fixtures still block
  delivery; existing landing restrictions, including grates, remain in force.
- Firebolt, Poison and Acid already stop at the first creature. Acid keeps its
  eight physical tiles. Lightning still pierces creatures and continues beyond
  the clicked tile. Mental/conjuration effects, Floods and Beckoning are unchanged.
- The UI shows delivery, actual known impact and affected area separately.
  Mouse and keyboard feed the same detached result and command. Invisible
  occupants cannot change preview or prepayment admission. Unknown terrain and
  incomplete portal links stop the displayed prefix; random forks show possible
  exits without promising a landing or area. Execution may encounter an unseen
  obstacle, and only committed traversal consumes the existing fork RNG.
- Travel, impact, crystal-area and unfolding cues use the actual resolved path
  or location. Animation never drives gameplay or changes action accounting.
- Sparkle has no lifetime countdown. Its existing serialized permanent-fire
  entry and brazier light remain until extinguished. In R52, placement on deep or
  shallow water with positive Ichor retains both. Ordinary water immediately
  removes fire, its source, light and relighting state. If Ichor is later absent,
  the existing shared wet-contact cleanup quenches the source. Hellfire and
  unrelated Ichor/fire rules are unchanged. The Sparkle cue reports an ember or
  extinguishing puff according to the actual result, not merely the water tile.

No new serialized state or migration is added. Existing save, Snapshot, Future,
replay, player-plane and active/offscreen propagation paths retain ownership.
Support identities and costs are unchanged. R52 Pack details and Sparkle prose
describe the revised behavior without changing frozen offer-weighting text.

## Deterministic regressions

Core amulet tests cover creature interception, wall/door/Borge/Epidite collision,
known preview and execution endpoints, shifted crystal animation cells, Blink
backtracking and range, invisible/unknown blockers, Lightning piercing, Acid
range, linked paths and serialization. Existing R50/R51 tests remain unchanged.
The restoration integration tests add Sparkle dry/ordinary-water/Ichor-water
placement, 32 independent propagation ticks, source removal without relighting,
correct ember/puff cues, mixed collision/Sparkle save-load replay, Snapshot and
Future restoration. Desktop tests cover matching mouse/keyboard input, actual
command receipts, cancellation, resized full/retained rendering and tile redraw.

Focused verification passed on 2026-11-07: 35 amulet unit tests, 14 restoration
integration tests and six desktop aiming tests. The integration fixture's first
compile used a private actor helper; it was corrected to the public accessor and
all 14 tests then passed. Core receipts are
`verification/revision52-enemy-followups/amulet-units.*` and
`verification/revision52-enemy-followups/ability-restorations.*`.

The final shared desktop batch also passed the three Speed Up regressions and
one each for merged bleeding meters and revision-scoped creature text. Its
unique receipt directory is `verification/r52-targeting-sparkle/native-attempt-01/`:
build and per-filter logs/exit codes, pinned compiler/profile, unchanged before/
after executable-input hashes, exact binary hash and 17 native PNG captures.
Ten captures show R52 intercepted/clear Iceblast, shortened/occupied Blink and
cancellation at 1280x800 and 960x1200; the other seven retain R51 examples.
The actual intercepted-blast and shortened-Blink landscape frames and the tall
intercepted-blast frame were visually inspected. The blast footprint is centered
on the interceptor; Blink ends on the preceding empty square.

These are focused native-raster/command checks, not native OS-input or Windows
execution receipts. Core tests preceded the separate final wound-boundary
validation change; the shared desktop binary includes it. The final frozen-source
workspace gate remains a separate required check.

## Complexity and bounds

Let V be floor cells (admitted maximum 3600), A living actors, P spatial
link/alias entries, M terrain-modification cells, G grate entries, F cloned floor
field size and Q existing observation/visibility cost. Reuse the existing bounded
portal planner and traversal: O(P²V + VAP + A²) launch time, O(V+A+P) scratch,
plus existing plane/owner helpers. Finite Iceblast does not extend its slope to
the map boundary; its area adds at most nine checks plus the existing actor scan,
damage and freeze work; alias-aware actor membership adds at most nine O(P)
equivalence queries per actor. Blink's reverse landing selection adds
O(V*(AP+M+G)) worst-case time and O(1) scratch. These are conservative source
bounds, not timings; bounded spatial aliases and floor size still apply.

Preview runs on aim/observation invalidation and once for Blink admission, never
on animation frames. It costs
O(Q + F + A*actor-visibility + P²V + VAP + VM), with O(F+V+A+P) scratch.
Blink landing filtering uses observed terrain/occupancy; Iceblast adds at most
nine known-cell checks. Unknown ordinary portal endpoints add O(P) sanitization.
Existing rendering, GPU/framebuffer cost and retained O(D) descriptor work are
unchanged; see the R51 aiming ledger for D delivery/area/impact/exit coordinates.

Sparkle adds only the existing O(1) Ichor/hellfire layer predicate. Successful
permanent placement retains its O(V+S+L) environment/source cost, where S means
permanent source count and L fixed lights. Quenching remains O(S+L) per
contact, O(V+S+L) per adapter entry with O(V) mask scratch, plus floor COW and
existing propagation. Save/load and time travel retain whole-state costs.
