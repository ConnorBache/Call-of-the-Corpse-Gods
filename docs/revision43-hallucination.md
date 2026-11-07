# Revision43 hallucination exposure and presentation

## Exposure implemented

Hallucinogenic gas refreshes Hallucinating only for the player and existing
shared-owner endpoints (Half Here and active Dreamwalking body). The existing
owner identity helper is used; ordinary enemies and allied NPCs no longer gain
this status or treat hallucinogenic gas alone as a hazard. Gas-immunity rules,
body-local conditions, two-world-turn refresh/decay, gas diffusion, shared sight,
and other gas effects are unchanged. Revisions29–42 retain their former rules.
No body-local condition is transferred into the player.

## Complexity

The new eligibility predicate is O(1) fixed owner/status checks with O(1) extra
space, invoked at the existing per-contact actor exposure and hazard queries.
It adds no actor scan, RNG draw, visibility query, or new persistence payload.
Existing environment simulation and actor-access costs are unchanged.

## Verification scope

`tests/revision43_hallucination.rs` covers old42/new43 enemy/allied exposure,
player refresh, shared-owner endpoint exposure without condition transfer,
immunity, unchanged exposure RNG, hazard routing, world-turn decay and save/load.
All four exposure tests passed in the lead-controlled focused3 gate.
Presentation implementation and its pending verification are documented below.

## Resolved presentation policy (revision43)

The designer chose constantly changing distorted creature identities, with a `?`.
The desktop creates six detached display variants from the existing disguise
catalog for already-admitted visible/sensed creatures. A shared 400ms cosmetic
sample cycles these variants; adjacent samples differ. Map glyphs, trailing-name
question marks, actor superscript `?`, left list, hover and inspection reports use
that same phase. Reduced motion uses sample zero. Self/avatar/body presentation,
inventory, menus, historical Chronicle and canonical actor names remain stable.
Terrain labels used internally as render discriminators stay unchanged; detached
terrain/item-on-ground observation prose is marked uncertain. No hidden occupant
is admitted to a cache, and no simulation RNG or serialized state is changed.
Replay43 uses the same cached cosmetic selection; older revisions keep their
existing tick-based projection and unmarked text.

### Cache construction and invalidation

A normal world observation builds six actor Cell/sidebar variants. One lazy,
short-lived sidebar Run scratch supplies projected effective stats/statuses; each
actor is restored in that scratch before formatting the next actor. This avoids
re-fetching the real actor's max-energy behind a fake displayed identity. Inspector
opening runs the existing terrain/visibility admission once, then uses one
short-lived detached Run scratch to format six coherent reports (name, stats,
abilities, attack estimates and secondary observed identities). Existing combat
preview helpers can themselves clone the scratch; no claim of a single total
clone is made. The cache retains only detached display payloads, never a Run.
Actual world changes/target changes rebuild these normal observation caches;
cosmetic selection does not. A View records actor-cell indices and its phase, so
idle eligibility is O(1), and an unchanged phase does not copy display payloads.
Full painting remains default. The optional tile painter marks old/new actor
footprints and the creature-list region; active inspection panels also redraw.

### Complexity and measurement contract

Let C be observed paint layers, A visible/sensed actors, S status entries per
actor, D display-payload bytes, and V the existing visibility/terrain cost.
Normal view construction retains its existing V and appearance costs, plus six
bounded-template actor projections per admitted actor. Current projection and
stats helpers may scan actor/catalog inputs; this is not an O(A)-only promise.
The six cached meter projections use O(6*S^2) label matching at world rebuild.
Report opening is V plus six existing formatter costs, including their combat
preview clones; it is not six V traversals. Retained extra memory is O(6*D + A)
for display variants and actor-cell indices, not six worlds. At a changed sample,
selection costs O(A + selected display bytes); unchanged samples are O(1).
Painting/upload still cost their existing visible framebuffer work.

Focused tests cover shared-phase names/glyphs/stats, meters, player exclusion,
reduced-motion cadence, hidden-state exclusion, old42 behavior, canonical/token
immutability and tile/full pixel equality at1x/1.5x/2x. With runtime-timing enabled,
100 cached samples must have no `vision.observe_terrain` span. The initial
retry2 gate passed6 tests (one benchmark ignored), including all three native
scales; the subsequent sidebar-energy regression awaits its retry. The ignored
`revision43_hallucination_cache_benchmark` emits20 independent raw samples for
42/43 with1/32 actually admitted actors, separating initial view construction,
initial hover-open report construction and100 cached selections. It does not
claim that cached selection includes raster/upload. Results remain pending lead
execution; no performance numbers are inferred from source.
