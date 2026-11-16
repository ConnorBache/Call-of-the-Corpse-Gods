# Complete enemy-perception outlines

## Requested presentation

The designer requested: “When seeing what enemies can see, it should draw your
full knowledge of space they can see. So there should be a complete, enclosed
red area.” This supersedes the intentionally open contour behavior documented
in `revision51-perception-ui.md`. The existing R51+ hover preview now traces the
complete perimeter of the confidently known enemy-sight area. It closes at fog,
uncertain knowledge, remembered-only uncertainty and map limits, including
internal holes and disconnected regions. The existing thin red line and hover
interaction are retained; no red fill, new control, color change or combat rule
is added. An excluded unknown cell is not a promise that the enemy cannot see it.

## Knowledge and mechanics

`perception_ui.rs` continues to derive a detached preview rather than querying
an enemy's live world mask and clipping away hidden output.

- Current directly observed cells retain their exact observed illumination.
  Effective enemy Perception, creature rather than tile range, both player
  Stealth terms, invisibility, target light modifiers, optical ray endpoints,
  known portals and identity/plane restrictions remain intact.
- Remembered terrain and remembered door state are also usable knowledge.
  They come from `TerrainObservation`, never the hidden current map. A door
  whose remembered open/closed state is missing remains geometrically uncertain.
- Remembered illumination is not stored. Such a target is tested across the
  entire possible illumination interval, −100 to +100, after the same target
  light modifiers. It is admitted only when it is visible at both extremes
  and both geometric bounds agree. For example, Perception 200 can establish
  sight of a remembered target at distance 20 even at −100 light; distance 21
  remains uncertain. Neutral remembered illumination is never invented.
- Unobserved intermediate geometry is tested as opaque and transparent. A
  disagreement is uncertain, not visible. Unknown destinations are never
  admitted, and their current walls, doors and light cannot shape the outline.
- Portal identities still require both endpoints to be currently directly
  observed. An exposed endpoint with an unknown mate continues to suppress
  the preview. Remembered terrain does not grant knowledge of a hidden link.
- The 2026-11-09 follow-up unions known special senses with optical sight.
  All Seeing, telepathy, an unexpired Came tracking this player and qualifying
  Witch senses enclose all known same-plane cells. Mind Read adds the exact
  Euclidean radius 5, or 20 while active, independently of walls and light.
  Dream See covers all known cells for a sleeping/resting/recovering target;
  while active against an awake target it adds only confirmed known optical
  routes, without the ordinary light/range restriction. Actual target life,
  identity, active-state and qualification checks are retained.
- Special senses are checked before invisibility/optical-light restrictions,
  matching actual sensing rules. Remembered terrain needs no remembered light
  for a sense that ignores illumination. Unknown cells remain excluded.
- Disguised/hallucinated creatures, sensed-only enemy identities and unsupported
  mixed planes remain excluded. Moving player illumination (Shine, Ring of
  Light or darkness-aura body) and an unknown portal mate disable optical
  evidence, but no longer erase independently known special sensing.
  Dream See's uncertain geometry remains omitted if a portal mate is unknown.
  This is a conservative known-sight area, not an assertion of safety outside.

The outline is the exposed sides of all confirmed-visible cells. Adjacent
confirmed cells share no internal edge. Every outer loop and every hole is
closed, including diagonal contacts. The right and bottom map strokes are kept
inside the map clip, and edge endpoints meet at corners. Painting and dirty
bounds use the same geometry. Full, retained and tile-damage rendering therefore
clear and replace the same old contour when hover or observation changes.

This is observation/presentation only: no simulation, enemy awareness chance,
turn consumption, normal/haste dispatch, equipment, offscreen behavior, RNG,
serialized field, save migration or time-travel history is changed. Current
observations after save/load, movement or either time-travel restoration use
this same stateless projection.

## Complexity and bounds

Let V be floor cells, A displayed eligible creatures, N floor actors, K admitted
portal endpoints (currently at most 14), R maximum ray length, B the existing
actor-stat, target-light, apparent-identity and equipment/ownership work, and
E emitted edges for one creature, bounded by 4V.

At world observation, current/remembered geometry and light bounds cost
O(V × B + K) time and O(V + K) storage. Current light is exact and remembered
light uses two bounded scalar evaluations. Displayed-ID lookup retains O(A × N)
actor scans, plus existing identity and stat projection costs. Special-sense preparation adds O(B) work per creature. All-known and Mind Read
checks are O(1) per known candidate and can avoid optical work. Active Dream See
adds at most one known-blocked route query per candidate, so the maximum three
route queries retain the same asymptotic bounds. Two optical bounds
for V candidates retain O(A × V × R) time without portals and
O(A × V × K² × R) with them; each portal route retains O(K) temporary storage.
Remembered destinations can increase the number of candidates requiring these
existing queries. No additional live geometry or source-light queries are added.

Closed perimeter extraction is O(V) time and O(E) output per creature, replacing
the previous O(V) interface scan. Observation scratch is O(V + K), with
O(A × E) detached boundary output. No tracing search, sorting or new cache is
needed; each exposed side is emitted once.

Pointer selection remains O(A). Paint emits O(E) native solid rectangles plus
their clipped native-pixel fill area. Native scale increases stroke coverage;
the GPU path records and draws those same rectangles. Damage production is
O(E) rectangles followed by the existing row-span union/coalescing, and world
comparison/copies remain O(A × E). Font and other map rendering costs are
unchanged. These are source-derived bounds, not measured performance claims.

## Regression coverage

The desktop `perception` filter includes the original nine perception suites
and new R52 checks for:

- All 19,683 ternary 3×3 masks, verifying no duplicate/open edges and exact
  reconstruction of the admitted cell union, including disconnected regions,
  unknown holes, false-cell holes, diagonal contacts and every map border.
- A closed internal hole and its four sides.
- Guaranteed remembered sight, uncertain remembered illumination, remembered
  wall occlusion, current-cell exact illumination and unknown-cell exclusion.
- Identical output after hidden wall/light changes, using both frozen detached
  knowledge and fresh observations with unchanged remembered knowledge.
- Every perimeter pixel and all four map corners at 1280×800, 1920×1080,
  960×1200, 1537×911 and 2560×1600, plus damage-bound coverage and hover clearing.
- Full/retained/tile repaint equivalence through fog, remembered extension and
  wall occlusion, repeated hover/cancel, scrolling, resizing and world replacement.
- An isolated SDL readback comparison at three sizes, requiring exact red-line
  pixels and the established maximum three-channel rounding tolerance elsewhere.

Set `CORPSE_PERCEPTION_REVIEW` to an absolute directory for native software and
SDL PNG captures. Run the ignored SDL check with
`SDL_VIDEODRIVER=offscreen` and `--test-threads=1`.

## Executed cloud verification, 2026-11-08

The locked/offline pinned Rust 1.99.1 desktop build passed. The shared profile
used optimization level 1, debug information 0, debug assertions and overflow
checks enabled, incremental compilation disabled and two build jobs.
`perception_ui` passed 13 tests with no failures; its one isolated SDL test was
then run explicitly and passed at all three native sizes. The ordinary suite
ran in 1.63 seconds and the isolated SDL suite in 1.29 seconds. These are test
wall times, not algorithm benchmarks.

The reviewed fixture screenshots show a complete contour at the unknown fog
limit, an extension into guaranteed remembered sight, a closed wall-occluded
shape, an internal excluded dark hole, and visible right/bottom map borders
including portrait/native fractional scaling. Eighteen PNG captures and the
compile/test logs are under `verification/r52-enemy-perception/`. The SDL
readback is an actual offscreen native renderer capture, not a simulated image.
Mesa could not create its optional home-directory shader cache and disabled
that cache; rendering and readback still passed.

This verifies the scoped overlay source. The integrating release owner runs
final aggregate checks after the separate chasm/corpse changes. Cloud software
and offscreen SDL checks do not establish hardware GPU or Windows runtime
behavior. No user PC, publication or deployment was involved.


## Follow-up investigation and expanded senses, 2026-11-09

The frozen Avatar World source and the current source both contained the
complete enclosed-perimeter implementation before this investigation. The
standard ordinary-sight regression was not reproduced. It was nevertheless
incomplete for special senses: the previous code returned no boundary at all
for All Seeing, telepathy, Came, Mind Read, Dream See and qualifying Witch
senses. That confirmed omission is now replaced with the knowledge-safe union
specified above. This does not establish which case the user's screenshot was
showing; their actual installed Windows build was not accessed.

Additional regressions use the original authored `avatar-room-v1.json`, changing
only its revision from 29 to 52. No terrain, light, Perception, actor or memory is
enhanced for that scene. Actual map/sidebar input (including the preceding stair
row), tooltip appearance/dismissal, cancel/reenter and animation updates run
through the native tile compositor and compare byte-for-byte with full paint.
The contour is checked against core actual sight on every known unobstructed
ray, and every visible contour pixel is checked on the unobscured sidebar path.
A tooltip legitimately covers map pixels; it is not drawn behind the contour.
This test also exposed a genuine portrait-window redraw defect: both retained
and tile tooltip damage helpers used fixed 1280×800 bounds while the painter
used the responsive viewport. They now use the painter's exact width/placement
and logical height, restoring complete tooltip/map redraw on open and dismiss.
Visual review then caught the companion paint defect: creature/item tooltip
backgrounds were themselves clamped to logical height 800. Their clips now use
the actual logical viewport, and a regression checks the exact translucent
background pixel below that old limit. These changes are O(1) geometry/clip work;
existing native-area raster work is unchanged.

Special-sense tests compare every cell of a fully observed fixture against the
core's actual sensing query for valid, expired, wrong-target, wrong-depth,
active/inactive, sleeping/resting/recovering and nonqualifying cases. Other tests
cover remembered cells, hidden mutations, an unknown internal hole, identity
concealment (including invisible and highly stealthed sensed actors on directly
known terrain), unknown portal mates, moving illumination and optical/sense union
around a wall. The tests alone may call live scalar visibility as an independent
oracle; production remains detached and never consumes that oracle.

The new authored-room test costs O(V times scalar core-sight/known-ray work)
plus 24 complete native reference paints, tile paints, and pixel comparisons.
Special-sense oracle tests add 16 fixed fixtures times V scalar core queries;
no such test oracle runs in the shipping game. Screenshot encoding costs
O(native output pixels) memory/time per image. Actual compile/test logs and
source/binary/image hashes are in `verification/r52-perception-recheck/`.


### Executed follow-up verification

Pinned Rust 1.99.1, locked/offline desktop compilation passed in 50.06 seconds
using the unoptimized test profile with debug information, assertions and overflow
checks, incremental disabled and two build jobs. The final scoped source passed
18 perception tests with zero failures (28.75 seconds), followed by the explicit
isolated SDL test with zero failures (10.50 seconds). These are cloud test wall
times, not performance benchmarks. All 29 CPU and 3 SDL PNGs were freshly
exported from the exact same pinned binary; source, binary and image hashes are
recorded in `verification/r52-perception-recheck/report.json`.

Visual inspection of the final authored room confirms closed ordinary sight,
readable portrait tooltip background and clean dismissal. The special-sense
images confirm a closed known-space perimeter through walls for telepathy,
limited Mind Read extension through walls, and wall occlusion for active Dream
See. Native SDL readback retains the complete map border and internal dark hole.
The sensed-only invisible/stealthed-actor regression passed after the independent
identity-admission review. Final combined workspace/release gates remain the
release owner's responsibility; no installed user PC or Windows runtime was read.
