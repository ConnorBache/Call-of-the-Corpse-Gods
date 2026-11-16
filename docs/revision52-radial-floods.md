# R52 Floods: a radius-four circle (deferred proposal)

**Not included in the released runtime.** The release split retains the previously
released targeted wave until the pending damage/push decision is answered. This
document preserves the unfinished proposal and its historical verification scope;
all present-tense implementation descriptions below refer to that separate
checkpoint. See [the retained release contract](revision52-released-floods.md).

## Authority and command compatibility

The designer requested deep water with outward currents around the player,
then clarified the shape as “A circle of radius 4.” The native Apply action now
emits `Command::Use` for R52 Floods. This action scans the integer cell-center
Euclidean disk `dx*dx + dy*dy <= 16`, containing 49 cells before clipping and
protection checks. It never asks for a direction or creature target.

Existing recorded `Command::UseAt` retains the historical directional wave,
including R52 shallow-to-deep consumption. R50/R51 remain targeted waves and
reject `Use` for Floods. This command distinction preserves old Floods journal
semantics without reinterpreting old saved targets. It is not a promise that
all older seed replays remain identical across unrelated R52 amendments.

The cost remains 30. R52's current reserve ceiling is 10, with the existing
player-energy fallback, blessings, cooldown, free-action and recharge rules.
The historical tier damage table remains available to recorded wave commands.

## Geometry and effects

Each candidate has a direct Bresenham line from the player. The lines do not
curl around walls, flood-fill rooms, or redirect through portals. Walls, trees,
chasm/pit, closed doors and genuinely solid Borge/Epidite cells block the line.
A diagonal step also requires its two side cells to be unblocked. These are
explicit bounded physical-line interpretations of the existing protection
rules, not a newly authored propagation system.

Ordinary floor, grass, snow, sand, shallow water and existing deep water may
become deep water. Stairs, open and closed doors, grates, portal endpoints and
aliases, permanent authored terrain, solid Godbeast structures and unrelated
temporary walls remain unchanged. Protected non-solid cells can be crossed by
the liquid's line, but are not overwritten; grates remain liquid-permeable.

The eligible center tile becomes deep water with code 0, because it has no
outward direction. Every other affected cell uses the closest of the eight
existing current headings. With `major=max(abs(dx),abs(dy))` and
`minor=min(abs(dx),abs(dy))`, choose the cardinal major axis when
`(major-minor)^2 >= 2*minor^2`; otherwise use the diagonal. This is the exact
integer comparison against 22.5 degrees, without trigonometry or RNG. Thus
(3,1) points east and (2,2) southeast. The 49 offsets contain no nonzero tie.

Water and currents last 12 normal world turns. Ordinary fire is quenched using
the existing helper; permanent/divine flames retain their protections. Current
movement, occupancy collisions, shoreline admission, flight, item drift,
normal/haste timing and environmental contact remain owned by the existing
runtime. Water under the player immediately affects the derived swimming state.
Currents move during ordinary normal-turn processing, not during animation.

**Pending designer clarification:** whether the new radial action should also
retain the old direct damage packet and immediate two-tile push. The radial
terrain/current implementation is provisional until that decision is recorded.
The historical `UseAt` wave's damage and push remain unchanged.

## Ownership, saving and expiry

The existing temporary-terrain record gains an optional `floodCurrent` object:
original direction, applied direction and whether its current grid originated
as Missing. Old records omit the field and retain their old wire shape and
expiry. New metadata is admitted only on R52 water records. Natural deep water
is an admissible predecessor only with valid new metadata. Null metadata,
invalid or fractional direction codes, duplicate cells, bad tiles, bad expiry,
out-of-bounds cells and wrong revisions reject before mutation.

Recasts coalesce records instead of stacking. They preserve the first terrain
predecessor while the old record still owns the live terrain and preserve the
first current predecessor only while it still owns the live arrow. Independent
later terrain/current edits become the baseline for a new cast. Expiry restores
each layer independently only when its live value still matches the effect's
applied value. This follows existing value-based terrain ownership: an external
write of the identical value is not distinguishable as a separate edit.

Missing and an existing all-zero current grid remain distinct. The first cast
allocates a grid only if at least one cell is affected. Missing provenance
survives overlapping, disjoint and refreshed casts. When the final overlay ends,
a grid that began Missing becomes Missing again only if every arrow is zero.
Unrelated nonzero arrows prevent that cleanup. If the entire grid was removed
between casts, recreating it invalidates every older arrow predecessor, including
zero-current centers, so a deleted layer cannot resurrect old arrows later.

New non-targeted dispatch preflight and amulet admission check current-grid
shape and values before payment or even the no-turn finish/memory path.
Persisted radial records also require a valid grid when a grid is present;
Missing is allowed because an independent later deletion must remain safe.
Typed command preflight, raw storage, Snapshot decode/restore and Future restore
cover the metadata. Snapshot checks occur before live state mutation. Existing
Dreamwalking consumable restrictions still block both Use and UseAt; serialized
dream-floor records nevertheless expire and restore in their own plane.

The optional preview helper gates each destination, line and diagonal-corner
cell through same-plane direct optical visibility before reading live geometry
or protection metadata. Other-plane viewpoints and telepathy cannot reveal
hidden player-plane geometry. Detached animation receipts use actual affected
cells and the existing presentation visibility gates. The immediate Apply UI
never enters the historical wave target mode.

## Complexity

Let C<=49 be disk cells, L<=5 a direct ray's cell count, V current-floor cells,
T temporary records, M permanent-terrain membership entries, P portal/topology
entries, G grates, B Godbeast protection work, D door-key lookup work, U the
existing direct optical visibility-query work (including cold cache misses),
and F the current floor's complete copy-on-write payload.

The geometry scan is O(C*L*(U+D+B) + C*(T+M+P+G+B)) time and O(C+L) scratch
for known-only preview; actual execution omits U. Portal helpers' list allocation
and membership scans are included in P. Current direction selection is O(C).
Painting and coalescing cost O(C*T+T+V) worst case, including initial grid
allocation or missing-layer ownership invalidation. First mutation may detach
O(F) data when a Snapshot shares the floor. These are per-cast costs.

The narrow dispatch guard scans O(I) inventory items for each Use command and
costs O(I+V) for an R52 Floods Use, with I the inventory size.
Pure radial admission and its defensive execution recheck each scan the active
current layer in O(V). Typed command and Snapshot temporary-terrain validation
add O(sum(V_f)) over waking and dream floors with radial metadata, in addition
to existing record/protection validation. They do not rescan unrelated old floors'
current layers. No permanent current-grid cache is introduced.

Expiry retains O(T) scan/storage and adds an O(V) zero-grid scan only when the
last overlay from an originally Missing grid is removed. Restoration receipts
retain existing per-cell visibility work. Additional serialized storage is O(T)
small current records, plus O(V) only when the floor needed its first current
grid. Save/Future/Snapshot encoding retains ordinary whole-state costs.

Native rendering and GPU uploads retain existing visible-cell, pixel and
retained-damage costs. Up to 49 cells replace up to 15 in the action receipt;
there is no new per-frame simulation, current-generation pass, pathfinding,
external-library call, or gameplay RNG call. Existing environment movement and
item drift retain their own costs and RNG behavior. These are source-derived
bounds, not measured performance claims.

## Verification plan and scope

The focused core suite covers all 49 heading/geometry offsets, center water,
clipped cells, solid barriers, protected ground, repeat payment and expiry,
legacy command semantics, old records, natural water/current predecessors,
independent edits and layer deletion, Missing versus zero grid, normal/haste/free
actions, offscreen expiry, Dreamwalking restriction/plane ownership, visibility,
malformed typed and serialized state, Snapshot and actual Future restoration,
and deterministic recorded-command replay.

The native suite uses real Use commands and observation. The optional
`CORPSE_R52_FLOODS_OUTPUT` directory receives landscape and portrait captures
at 1280x800 and 960x1200. Full, retained and tiled rendering are compared byte
for byte for radial water, currents, recasts, immersion, restored ground and
actual animation removal. Historical mouse/keyboard target preview comparisons
exercise retained UseAt wave compatibility, not a new radial target mode.

Executed receipts and remaining release gates are recorded by the final source
verification owner. A focused core pass is not a full workspace or Windows
runtime certification.
