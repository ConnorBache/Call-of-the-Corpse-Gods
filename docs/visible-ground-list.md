# Visible ground entries on the left

The left column now starts with visible Upward/Downward Staircases, followed by
Monsters, Items, Vents, then Corpses.
Each group is nearest-first using the existing Chebyshev tile distance. Ties use
row/column and stable entity identity, then name. Monsters retain their existing
distance/ID ordering and full resource/condition cards. Their detached behavior
labels (Hunting, Sleeping, etc.) are restored; sensed actors retain only their
already-admitted Sensed label rather than disclosing a live behavior.

Ground rows are collected during observation from current direct evidence,
separately from map glyph z-order. Stacked items and visible corpses beneath an
item remain separate entries. Remembered/detected-only items and unseen vents or
corpses are not admitted. Existing item naming/identification and dream-plane
filters remain in effect. This changes presentation only, not save state,
turns, simulation, or discovery. The column stays on the left as requested.

Rows share measured paint/hit/scroll geometry. Explicit mappings distinguish
combined row indices from creature and ground indices; staircase prefixes cannot
be mistaken for creature indices. A prefix partition lookup is O(log G), and
card reordering/top-offset calculation is O(A + G) after measured layout.
Painting resolves two mappings per card, adding O((A + G) log(G + 1))
comparison work; each individual pointer/scroll mapping is O(log(G + 1)). Ground hover uses an observed tile
coordinate, never a counterfeit actor ID. Hover highlights both row and map tile;
right-clicking a ground row opens existing tile inspection. Creature right-click
inspection remains unchanged. All rows sharing one tile highlight that tile.

## Complexity

On an existing observation pass, C terrain cells, I ground items and A admitted
actors add O(C + I + A) predicate/collection work, plus existing item-label and
profile lookup costs and P total corpse-presentation helper work. Corpse projection
reuses the existing apparent-identity adapter; each call may scan shared origins
and construct six template variants while hallucinating, so P is not treated as
constant per actor. A conservative bound is O(A² + A × K) when origin lookup
scans A actors and K bounds template/profile projection work. G admitted ground entries sort in O(G log G) comparisons;
string comparison can cost O(L) on exact category/distance/coordinate/ID ties.
Extra detached storage is O(G + T) for G entries and T label/glyph bytes. Existing
visibility projection is reused, not rebuilt for each entry. No new world scan
runs on animation frames or pointer movement.

Card paint/hit-layout remains worst-case O(T²) measured wrapping plus native
font rasterization; content storage is O(T + A + G). Hit testing is O(A + G),
scroll clamp/reveal O(1) after layout. Pointer map resolution scans detached cells
and entries O(C + A + G), and retained/tile selection invalidation marks the two
old/new coordinate footprints plus the column. Snapshot comparison/copy includes
O(G + T) ground data. No persistent unbounded cache is added. Rendering costs
remain proportional to clipped text/rectangle pixel coverage. These are analytic
bounds, not measured timings.

Verification is in enemy_list::ground_tests, existing enemy-list tests, and the
combined main integration patch. At initial validation the authoritative main
source was still being recovered, so runnable checks used a disposable harness
with an explicitly labeled pre-existing stat-presentation compile bridge. Those
checks do not certify the recovered final release or Windows execution.

Hallucinating ground labels receive the same uncertainty suffix as tile inspection.
Corpse identities use the actor adapter's detached six-sample name/glyph/color palette,
including corpses hidden beneath item glyphs; animation selects that palette
without querying the world. This adds O(G) selection work and a fixed sixfold
name/glyph-byte bound per hallucinated corpse. Physical directly observed ground
entities appear once; duplicate portal/pocket representations are not extra rows
(the existing alias adapters require the original entity to be directly visible).
