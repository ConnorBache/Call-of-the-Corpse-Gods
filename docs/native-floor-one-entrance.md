# Persistent floor-one entrance (native revision 31)

The designer requested a bottom-center entrance door at the beginning of every
run, shown as uppercase Ω in bright blue on a dark-blue background. This change
adds the entrance and its identity, not an escape or victory condition.

## Generation and compatibility

Normal native maps are 79×29. Revision 31 supplies the existing generator with a
floor-one up-stair anchor at (39,23). That is the generator's lowest permitted
five-cell inset and retains its ordinary entrance-room, machine-footprint and
vent exclusions. After generation and before population, the deterministic
entrance pass places the sole floor-one up tile at (39,27), the center of the
bottom interior row. The outer boundary remains closed.

The pass connects the new door to the generated entrance room with a cardinal
route, minimizing changed terrain first, then path length, with a stable
row-major tie break. Its first step must be north. The player starts at (39,26),
so the door can be seen immediately south of the player rather than hidden by
the player glyph. The ordinary population rule already excludes living actors
within five cells of the sole up tile; the pre-generation anchor protects this
arrival area from authored machine placement. No new spawn balance, equipment,
resources, or monster behavior is introduced.

The connector does not cut through authored machine footprints, active hazard
layers, vents, grates, permanent fire, braziers, other protected fixtures,
Godbeast rooms, or the down stair. The former entrance's own recessed wall frame
may be opened. Only a former stair torch actually crossed by the corridor is
removed; unrelated lights remain. The former up tile becomes ordinary floor,
and its obsolete stair-recess metadata is removed. A converted water tile has
its current cleared. If either required entrance/arrival cell contains an ordinary (non-machine)
room door, that door and its room/connector references are removed because the
new entrance corridor replaces it. Ordinary generated torches/braziers on these
two cells are moved to the nearest safe existing terrain, with stable row-major
ties. Braziers must satisfy existing gas/fire separation and component-access
checks without burning new vegetation or blocking a route. The initial five-cell
entrance neighborhood is excluded from relocation. If no safe site exists, only
that conflicting ordinary light is omitted. Permanent-fire and existing fire
layers/provenance are reconciled; all other doors and lights remain untouched.
Authored machines, vents, grates and unsupported fixtures are never cleared.

A draft floor makes this pass atomic. If safe planning fails, no part of the
supplied floor is modified; normal startup also has its existing run-level draft
boundary.

Generation consumes no additional random draws in the connector itself. The
new anchor intentionally changes revision-31 floor-one generation. Revisions
29 and 30 keep the original generator inputs, random streams, entrance location,
population, saves and replays. Existing legacy floors are not migrated or
retrofitted. Higher-floor entrances remain ordinary up stairs.

## Identity, saves and observation

`Floor.entrance` is an optional serialized `{ "x": 39, "y": 27 }` record. Its
presence identifies the door separately from ordinary stairs without adding a
new terrain kind. Legacy records omit the field. Explicit null, non-finite,
non-bottom-center, non-floor-one, and mismatched terrain identities are rejected.
The record is retained by normal floor serialization and Arc-backed snapshots,
including Revert and See the Future restoration. It survives leaving floor one;
visibility does not create or delete the identity.

The observation layer exposes the identity only with known map cells. Painting
uses the already-redacted observation, not hidden world data. The dedicated
appearance uses Ω, foreground RGB (76,170,255), and background RGB (8,25,82).
The normal fog-of-war contract remains in force.

An attempted ascent from floor one still returns without travel or changing the
run. There is no win, escape, unlock, or return-to-title mechanic in this change.

## Complexity and bounds

Let V be the grid cell count and M the total expanded setup/fixture/protected
metadata cells. The supported installer bound is V≤3600 (the normal map has
2291 cells). The pass runs once, only while initially creating floor one under
revision 31. Protection and hazard masks cost O(V+M) expected time, with ordinary
IndexSet/hash-map membership assumptions and bounded 13×13 setup footprints.
Stable four-neighbor Dijkstra costs O(V log V), including heap insertions and
pops; path reconstruction and terrain updates cost O(V). Total expected time is
O((V+M) log V), conservatively including numeric-key construction/hash costs,
and extra memory is O(V+M), including the atomic floor draft. In the exceptional
case of L conflicting ordinary lights (L≤2), sorting candidate destinations adds
O(L×V log V); each brazier candidate includes O(V+M) existing gas/fixture and
component-access work. The conservative total worst-case relocation bound is
O(L×(V log V + V×(V+M))), with O(V+M) reused scratch space. At most two graph builds
are needed, each O(V). Torch candidate checks are local after metadata masking.
The helper never runs per frame or simulation turn. The caller's existing
atomic run draft is separate and retains its preexisting costs.

Identity lookup and arrival lookup are O(1) time/extra memory, including a fixed
number of layer checks. Serialization adds one constant-sized optional record
per floor-one save/snapshot; existing full-save and temporal restoration costs
are unchanged. Native glyph rasterization, cache-miss costs, and drawable
painting are part of the existing renderer, not claimed constant wall time.
These are asymptotic analyses, not measured frame timings.

## Verification

`sh tools/cargo.sh test -p corpse-core --test floor_one_entrance` covers:

- Bottom-center placement, north landing, closed border, sole up tile, connected
  down stair, deterministic repeat application, and JSON round trip.
- Fixture avoidance, refusal without partial mutation, and unchanged lower
  floors. Adversarial ordinary lights/doors on the mandatory cells verify narrow
  replacement, safe brazier relocation, fire-layer consistency, omission only
  when no safe light site exists, and refusal to alter an authored machine.
- Rejection of malformed identities without treating old up stairs as doors.
- One hundred twenty seeded floor-one generations across all six domains.
- Normal revision-31 player placement, unoccupied arrival, save/reload, seeded
  replay/restart, floor travel away and back, Revert and See the Future
  restoration, and absent escape logic.
- Twenty-four additional fully populated floor-one seeds verify the north
  arrival remains safe and unoccupied after creature/environment initialization.
- Original floor-one entrance geometry and omitted metadata for revisions 29/30.

Existing frozen-v48 generation, startup and replay differential suites remain
the legacy compatibility gate. The complete native workspace suite is the
integration gate. Actual Windows rendering and input remain platform checks,
not conclusions from simulation tests alone.
