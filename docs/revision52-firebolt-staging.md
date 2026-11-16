# Firebolt terrain and projectile landing presentation

Firebolt simulation and its complete turn still resolve synchronously. The desktop
captures an outgoing observation only for an actual revision-52 Firebolt UseAt,
Shoot, or non-potion Throw command. It retains terrain and pre-existing fire layers only along the admitted
A02 Travel receipt. No hidden cell is queried or restored.

Until the advancing head arrives, the painter uses the old observed terrain/fire
at that route cell. Current actor layers, health/death outcomes, visibility, UI
inspection, cached View and its terrain observation remain canonical. Cells no
longer directly observed are never restored. Custom animated terrain reconstruction
uses a separate detached visual snapshot and excludes newly ignited, known terrain
fire emitters ahead of the head. It does not remove actor emitters or infer unknown
source origins. Nearby gameplay illumination/visibility remains the post-command
observation, rather than fabricating an incremental simulation.

Head arrival and the visible travel effect share the same receipt cell index and
clock. Queued effects begin with the first presentation frame after synchronous
work. Interruption, reduced/instant motion, route completion, changed floor/plane/seed
or missing cues discard the visual mask. Ordinary moves, melee and other amulets
never capture this snapshot. Recoverable physical projectiles use the separate
endpoint-only masking described below.

Replay uses the same staging. Autoplay waits until non-instant traveling effects
have finished, including an as-yet-unpainted queued frame. Manual stepping remains
an immediate interruption. Replay keeps its mask cache outside the shared painter
between paints so it cannot contaminate live play or another screen.

## Recovered physical projectiles

A completed shot or throw can already have placed its recoverable item on the
floor while its animated glyph is still in flight. The existing optional receipt
item_id identifies that exact projectile without changing any Run/save fields.
The desktop observed item layer carries the matching detached ID.

Until the final admitted route cell is reached, only that exact new item overlay
at the directly observed endpoint is painted as its current terrain backdrop.
Pre-existing observed IDs, actors, corpses, other ground items, visibility and
inspection remain canonical. Actual identity, not glyph equality, supports custom
item appearances. No receipt, missing source admission, consumed projectile,
unknown endpoint, or a scattered/fallen landing outside the admitted endpoint
produces a mask. Potion effects are never rolled back or delayed.

## Complexity

Let N be observed grid cells, V detached paint layers, R admitted Firebolt route
entries, K Firebolt travel receipts, E queued effects (existing cap 4096), and L
observed light-source footprint entries. I is carried items, G observed ground
rows, and H admitted recoverable physical endpoints (normally at most one). There is no new route
length cap or hidden route reconstruction; existing validated floor dimensions
bound N and each admitted route. Strings/glyphs and source-footprint vectors are
included in the copied bytes, not treated as constant-size records.

- Command admission adds O(I), O(1) scratch. Only the listed travel commands invoke the existing
  complete observation pipeline before dispatch; its visibility, light, actor,
  terrain and cold-cache costs are unchanged and are additional to the costs
  below. The temporary outgoing View is dropped after capture.
- Capture adds O(N + V + R log R + E) time, O(N + R) temporary indices/retained
  entries, plus copied route terrain/fire strings. Ground-ID collection and
  admission add O(G log(G+1) + E log(G+1)) time and O(G) retained IDs. Only O(R+G)
  old layers/observed cells/IDs remain retained after capture.
- Unchanged presentation frames compute arrivals in O(K*E + R log R), using O(R)
  scratch. K is normally one for a commanded Firebolt. Physical endpoints add
  O(H*(E+G) + H log(H+1)) checks/sorting and O(H) scratch; changed masks add
  O(H*V) exact item-layer/backdrop lookup. A changed withheld set,
  canonical observation identity, or hallucination phase additionally copies
  O(N + V + L) records/footprint entries and their string bytes. Layer membership,
  prior-layer lookup and source filtering add O((V + R + S) log R), for S detached
  source records. Per-frame painter access to the
  prepared cells/snapshot uses O(1) Arc clones, not full View/snapshot copies.
  The prepared visual cache retains O(N + V + L) records/footprints and their
  string bytes until it is replaced or the staging ends.
- Arrival changes conservatively invalidate/reindex native tile damage and repaint
  a full frame. This adds the existing O(N + V + footprint work) planner setup and
  full native raster/upload work for at most one transition per admitted route
  cell, plus interruption/profile/observation changes. Other frames retain ordinary
  effect/terrain animation damage, plus O(R) old/new staged-layer footprint
  queries and their bounded native raster area so larger old terrain glyphs
  cannot escape damage tracked for smaller final fire glyphs. Native glyph/font
  cache misses and animated
  material/light reconstruction retain their existing costs.
- Replay autoplay admission scans O(E) cues and uses O(1) scratch. It never delays
  gameplay mutation inside a recorded command or changes replay inputs.

These are analytical bounds, not measured speed claims. Focused native tests cover
advancing grass ignition, canonical view/save immutability, actor preservation,
visibility/plane loss, interruption, reduced/instant behavior, exact recovered
item identity, custom item glyphs, pre-existing/scattered ground preservation,
cache reuse, default and
custom animated-light tile/retained/full pixel equality, and replay pacing. They do not
assert OS event-delivery, Windows GPU presentation, or platform verification.
