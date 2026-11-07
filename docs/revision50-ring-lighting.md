# Revision 50 ring illumination

This implements the designer's fixed Light and Stealth values; no tier scaling,
rarity, or production item-generation distribution is inferred. R50 release
admission is still closed. `revision50_rings` identifies the equipped ring power
using the existing serialized Item.effect field, independently of display names.

- Light adds +10 to final environmental illumination at every tile in optical
  line of sight, except the wearer's own tile. There is no distance falloff or
  extra radius. Walls, closed doors and portal routes use existing optical LOS.
  The entire wearer tile is excluded, including any colocated other actor.
  Existing illumination limits remain −100 to +100. Each distinct worn Light
  ring adds +10, explicitly overriding the earlier duplicate rule for this ring.
  One owner's Half Here bodies contribute the same rings once across their
  combined LOS and exclude both occupied tiles. Unequipping, movement, death and
  plane changes affect the effect immediately.
- Stealth multiplies negative actor illumination by 1.2 once. Zero and positive
  illumination are unchanged. It does not darken the terrain or grant invisibility.
  It combines with existing Shadow in the Dark, followed by the established
  −100/+100 clamp. Independent stats-only Mini-Me copies have neither ring power.
- Cached tile fields and batched actor visibility follow the same calculations
  as scalar detection. Exact dependency capture observes equipment, position,
  opacity and portal edits within a single tick. Cache contents are not serialized.

## Darkness Aura

The designer explicitly confirmed that additive Light can raise −100 to −90.
R50 therefore applies the ring component to the aura bearer's absolute −100
actor illumination as well as to the surrounding tile field. Two worn copies
supply +20. The wearer's tile exclusion still applies. Historical revisions keep
the absolute −100 actor override unchanged.

## Complexity

Let N be floor tiles, A local actors, I the largest owner inventory, H active Light
emitters, K portal endpoints, R maximum ray length and B ability/status entries.
Every scalar tile-light query adds O(A*I + H*(F+K²R)) worst-case work, where F is
floor lookup cost (without portals, one O(R) ray per emitter). H is bounded by A,
not by the number of duplicate rings. Per-owner item counts provide magnitude;
shared-body LOS is unioned and does not duplicate the same physical items. Existing fire/shine/darkness work remains.
Stealth adds one O(I) inventory query to actor-light computation, with O(1) scratch. Scalar ring emitter grouping retains O(H) handles.

A cached immutable lighting pass captures ring membership in O(A*I), one optical
geometry dependency check in O(N+K), and H masks in O(HN). A cold geometry build
costs O(KNR+K); each cold emitter field costs O(K²+KR+N*(R+K)). Warm field lookup
is O(8) plus the O(N) mask projection. Retained ring dependencies use O(HN) memory plus an O(N) additive-component field
across at most two planes; optical cache bounds are unchanged. Building the light
field adds O(HN) operations after existing lighting. Tile lookups remain O(1).
No measured performance speedup is claimed; the focused tests check cache/scalar
agreement, direct edits, restore, portals, planes and legacy revision boundaries.
