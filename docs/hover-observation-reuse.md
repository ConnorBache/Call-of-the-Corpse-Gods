# Hover observation reuse

## Scope
Revision46 presentation-only change. No simulation, RNG, save wire, ability, balance, aesthetic or bow-control changes. Source baseline is the wording-refresh revision46 playtest. Full painting remains default.

## Corrected baseline
The old timing_test::hover_pipeline explicitly ran two inspections, two routes, and a full gameplay observation. The shipped MouseMotion branch instead already uses cosmetic_dirty and performs one inspection and one route when the hovered cell changes. The old8.305ms median on the normalized live tick48 fixture is a measured synthetic sequence, not the cost of every isolated current hover. It remains preserved as historical evidence, not used as the before arm here.

## What changes
A current gameplay View already holds an immutable TerrainObservation. Inspection, linked-portal actor aliases, non-Euclidean aliases and exit previews can reuse that exact observation rather than asking vision to build it again. Existing visibility, current/memory and actor-admission checks remain. Bow previews retain the original targeting and damage forecast logic; only initial terrain inspection is reused.

The route remains independently calculated using its original terrain_visibility mask and Knowledge::observe. No terrain-tile or cross-plane routing substitution is made.

Main allows reuse only when dirty is false and cached_view exists, or immediately after building a fresh View. Dimensions defend shape but are not a freshness certificate. A same tick/seed/depth is never used to justify reuse. Queued motion after a gameplay or geometry change defers expensive hover work until that fresh View. Cheap current screen-to-cell coordinates remain available immediately for same-queue Inspect; pending logical coordinates are remapped once the fresh View exists. Stale reports and paths are cleared on that deferred path.

## Complexity
Let V be current-floor cells, A relevant actors, F stored floors, H hazard/portal preprocessing and C_report the existing detailed report work. Supplied inspection avoids one full terrain-observation construction; it still performs C_report, actor/visibility queries and dream projection as before. Linked-space aliases avoid nested terrain reconstruction but retain their actor predicates. Current snapshot reuse adds no second retained terrain allocation: View already owns it. The original route retains its existing preprocessing and graph-search complexity, including O(V log V) heap work in applicable routing modes. Screen-to-cell projection is O(1) after floor lookup; deferred current-floor lookup is O(F), not a whole-world scan. Same-cell motion already did no report/route work and is not claimed as a new improvement.

## Verification and measurement
Exact report, pointer descriptor and fixed-clock framebuffer comparisons are performed outside timings, along with canonical Run equality. Cases cover current/memory/hidden terrain, same-tick changes with a fresh snapshot, portals/non-Euclidean endpoints, Dreamwalking/Half Here, hallucination variants, bow forecasts, shape mismatch and queued pointer cancellation. Benchmark headline uses collector-off alternating order, n>=30; separate collector-on diagnostics count inspection, route and terrain calls. These timings do not include the SDL event pump, GPU upload/present, OS scheduling or Windows execution. Tests that model helper calls are not claimed to execute the complete SDL event loop.

The historical CLI row is now explicitly labeled `legacy_synthetic_double_hover_plus_full_refresh`; old raw reports are preserved unchanged. The final diagnostic-only rename occurred after the optimized hover benchmark and does not affect its projection path.
