# Revision 51 perception presentation

R51 creature hover and the hovered left creature preview show `Perception range: N`,
using the authored neutral-light **tile** range `20 + Perception / 5`. The red
map contour instead follows plausible **creature** sight: twice the player's
functional distance, including their current Stealth and submerged Stealth.
The displayed base value is not a promise of a circular creature radius.
Positive target light at least +1 bypasses distance but not known line of sight.
Current tile inspection shows only negative contributions, such as
`−3 perception range` for −15 light. R50 and older retain their old labels.
The creature heart preview uses revision-aware heart rewards.

## Ownership and knowledge

`perception_ui` builds detached edge geometry in `View` on observation. The
painter selects it through the existing two-way `enemy_hover` identity, shared
by map and left preview hit testing. The sidebar reserves stable line space but
paints the range only for the hovered creature, so hover cannot shift hit targets. Painting, hovering and scrolling perform
no simulation queries, source scans, random draws or hypothetical actor moves.

The contour uses current directly observed terrain, doors and illumination.
It never calls the actual-world enemy mask and then merely clips its output.
Two route bounds treat unobserved intervening cells as opaque versus transparent.
Only outcomes on which both routes agree are classified. Edges separate known
true and false outcomes; no line is drawn against fog, remembered-only cells,
map limits or uncertain paths. Known portal pairs use the same core optical
routing algorithm on a new observation-only geometry object.

A currently exposed portal with an unobserved mate suppresses the preview.
Unobserved portal pairs cannot influence the contour. Current observed light
can legitimately change when visible illumination changes, even if the emitter
is not identified; no hidden emitter position or live source calculation is
used. Actor-only Shadow and Stealth-ring light modifiers share core logic.

The preview omits uncertain cases instead of revealing underlying identity or
mixing worlds: disguise/hallucination, active Dreamwalking or observed allies
from another plane, the exceptional player darkness-aura body, emitting Shine or a worn Ring of
Light. Observer All Seeing, telepathy, Came, Mind Read, Dream See and Witch
senses also suppress the optical-only contour. This is a
known conservative limit, not extra all-map sight. Player emitters are omitted because their moved light cannot be inferred from
the current detached field. Other supported cases use the current observed
lighting field with the player's current target modifiers.

## Complexity and bounds

Let V be current floor cells (supported 79×29 or square arrays), A admitted
creatures, N floor actors, K known portal endpoints (at most 14), R maximum ray
length, B player/observer ability and equipment modifier work, and E stored
edges per creature (at most 2V). At each world observation:

- Geometry/target-light preparation is O(V×B + K), with O(V+K) scratch. Modifier
  helpers can scan player equipment and shared-body ownership; B includes that
  work, not just ability count.
- Resolving A admitted IDs currently scans N actors: O(A×N). Effective observer
  stats and apparent-identity checks add their existing actor/catalog costs.
- Two optical bounds per candidate are O(A×V×R) with no portals, or
  O(A×V×K²×R) with portals. The core graph helper allocates O(K) route scratch
  per call. Total stored detached edges are O(A×E), plus O(V) state scratch.
- Pointer updates select a boundary in O(A). Paint is O(A+E) rectangle commands
  plus clipped native-pixel fill area. The existing GPU path records the same
  ordered solid quads, with no CPU raster of the frame.
- Hover changes invalidate old/new edge footprints. The tile planner adds O(E)
  rectangles and its existing per-row bitset union. Retained comparison copies
  O(A×E) descriptors on world replacement and uses its existing bounded damage
  coalescing. Animation redraws use the same contour pass and clipping.
- Text adds one bounded line per creature. Existing measured wrapping/font
  cache behavior applies (including cold glyph raster cost).

These are source-derived bounds, not measured performance claims. No new cache,
serialized state, gameplay RNG use, or simulation mutation is introduced.

## Verification

`revision51_perception` desktop test filter covers labels/legacy gating, heart
preview, creature-vs-tile radius, player stealth, darkness, +1 light, invisibility,
known walls, unknown intervening geometry, hidden wall/light changes, portals,
two-way hover, clear/cancel, scroll, resize, animation clocks, world replacement,
full/retained/tile pixel equivalence and GPU command-only output.

Desktop compile artifact 6 passed **9/9 focused perception tests**, with zero
failures or ignored tests, in 2.96 seconds. This includes the corrected
`cave-bat` fixture and revision-aware heart preview. Evidence is in
`verification/r51-ui/targeted-perception-aiming-1.log`; the exact binary hash is
recorded in `verification/r51-ui/desktop-compile-6-artifact.sha256` and the
perception verification manifest.

Six fresh software captures are in `verification/revision51-perception/`:
1280×800, 1920×1080, 960×1200 and 1537×911, plus map hover and negative tile
inspection. They match the previously reviewed pixels byte-for-byte. Set
`CORPSE_PERCEPTION_REVIEW` to an absolute output directory to regenerate them.
These captures precede the separately requested illumination color-curve
calibration; refresh them after that rendering work. The contour rules do not
change as part of that separate color calibration.

This is focused verification, not a full-workspace suite pass. Software captures
and GPU command-only checks do not establish hardware GPU or Windows behavior.
