# Revision 51 direct root lifecycle and material color

The designer reported that roots appeared to overgrow rather than follow the
most direct route, and shone neon purple. This correction changes root geometry
lifecycle and material color only. Two corpses is the initial generated feast,
not an ongoing acquisition cap. Gas identity, volume formula and acquisition
range are unchanged.

## Reproduction and cause

Two separate native reproductions are retained under
`verification/revision51-root-cleanup/`:

- `native-acquisition.json` is a controlled two-corpse input to the existing
  native `trace_core` executable. Its one real normal turn generated exactly
  two root paths, of 9 and 7 cells, in `native-two-targets.json`. Both were
  shortest Bresenham lines and both had color `#ee6cff`.
  `before-native-two-targets.png` is the actual 1920×1080 desktop renderer output.
- `before-stale-and-purple.json` is a validated controlled saved-state fixture
  with two live and two severed links. The old severing rule had removed only
  each cut cell. Its native `before-stale-and-purple.png` capture shows the
  remaining gray branches as well as the vivid purple live branches. It is a
  controlled lifecycle state, not a claim that the pictured cuts were performed
  interactively in that capture.
- `after-same-stale-fixture.png` renders that identical saved fixture with the
  corrected native desktop binary after compatible-load normalization. Both
  severed branches disappear and both live direct paths remain olive.
  `after-same-stale-fixture-observation.json` confirms RGB `[139,150,117]`
  (`#8b9675`) on all 12 interior live-path cells and floor dots on all 14 interior
  severed-path cells. These are native outputs, not retouched images.

The route algorithm itself did not wander: `fungi::root_path` already emits a
shortest direct raster line. The apparent extra growth came from retained
inactive paths and scalar root material left everywhere except the cut cell.
Observation still drew those inactive paths; old and live orientations could
also merge into junction glyphs. Retained links did not validate current endpoint
positions, and duplicate imported pairs could add redundant paths or gas counts.

Purple was explicit material assignment, not a GPU lighting problem. Revision
50 reused the hallucinogenic gas color for every fungal connection, and the
renderer deliberately used that supplied color when the user had not customized
the root appearance. The authored root material default is olive `#8b9675`.

## Correction

Revision 51 uses that authored olive for newly generated, newly acquired,
retained and observed roots, independently of the still-hallucinogenic gas.
Explicit appearance customizations continue to work.

A normalization pass preserves the first stored record for each fungus/corpse
pair, including inactive history. An inactive first record wins over a later
active duplicate, so a saved severed pair cannot reconnect accidentally. A live
record must still join its living fungus to its dead corpse, at the current
endpoint positions, with the exact shortest original path, intact root material
and no burning cell. A moved/dead endpoint, malformed detour or missing segment
severs the record; it is not rerouted or regenerated.

Only cells supported by surviving active connections retain scalar root
material. Shared cells survive when another connection still needs them. The
cleanup never creates a missing root cell. Inactive pair records and their old
paths remain stored to preserve reconnect prevention, but no longer draw or
leave orphaned sticky material.

Cleanup runs before active/offscreen fungal ticks and through the existing
compatible-load normalization of every retained temporal state. Fire, successful
root escape, corpse relocation, fungal source death and terrain collapse also
clear disconnected material immediately. Existing generation fire safety runs
before population roots exist and is unchanged. Earlier simulation revisions
keep their old color, material and replay behavior.

Observation draws only active connections. The existing revision-51 outgoing
segment orientation, final-segment continuation and intersection treatment are
retained. Hidden remembered cells are not rewritten or inspected during cleanup;
they keep their last observed appearance until seen again.

## Complexity and ownership

Let A be actors on the affected floor, E their inspected ability entries,
V<=3600 floor cells, K saved connection
records, P their total stored path entries, R<=11 cells in a valid radius-10
canonical path, D<=40 stored floors, and S the detached COW floor payload.

- A tick normalization builds source/corpse lookup maps and a stable first-pair
  set, validates every record and reconstructs only the support mask for scalar
  material. Expected time is O(A+E+V+P+K*R), scratch O(A+V+K+R). It includes endpoint
  lookup, ability classification, canonical path allocation/comparison,
  material/fire scans, stable duplicate removal and the full root-layer pass.
  Authored ability-ID comparisons have a fixed bounded string length. Hash
  collision worst cases add O(A²+A*K+K²) membership work; no output order
  depends on hash iteration.
- Orphan cleanup alone is O(V+K+P) time and O(V) scratch. Fire/escape already scan
  connections for the cut cell and now add that cleanup at revision 51. Corpse
  relocation and source death add it only on affected floors. Source death
  scans D floors and their root metadata, O(D+Ktotal+Ptotal+Vaffected), with no actor
  scan or RNG. The final root-layer mask never restores a missing segment.
- Compatible loading adds the sum of floor-normalization costs across the loaded
  state and each already-supported history/Future state. Existing serialization,
  anchor memoization, actor indexing and O(S) COW detachment costs remain
  additive. No new save fields, long-lived cache or random draws are introduced.
- Root observation still builds its O(P) expected index; skipping inactive paths
  and choosing a fixed color add O(1) per record/cell. Native glyph rasterization,
  font-cache misses, material grading and framebuffer upload are unchanged.
  A gas-colored background may still be purple because the gas is unchanged.

These are source-derived bounds, not measured performance claims. Profile large
saved histories with many severed records separately from ordinary fresh two-meal
encounters. The existing gas, LOS, path and actor work remains additional.

## Verification and limits

`revision51_root_cleanup` covers four simultaneous targets without a two-target
cap, exact shortest paths and stable repeated acquisition, olive material with
unchanged gas, full branch removal with shared-cell preservation, duplicate
records and inactive-history priority, stale imported scalar cells, load
idempotence/RNG, moved or revived targets, moved/dead sources, immediate fungal
source-death cleanup, legacy behavior and hidden-memory refresh.

Set `CORPSE_ROOT_REVIEW=verification/revision51-root-cleanup` when running this
target to export actual native simulation states for four live branches,
post-severing and source death. Render those files with the updated desktop
binary via `--fixture FILE --render-game IMAGE --render-size 1920x1080`.
All nine `revision51_root_cleanup` regressions passed against the coordinated
native core artifact (`verification/r51-ui/direct-core-tests.log`). Its actual
simulation exports were rendered and visually inspected with the updated native
desktop binary: `after-four-live.png` shows four direct olive paths,
`after-severed.png` removes exactly one branch while retaining the other three,
and `after-source-death.png` shows no roots immediately after source death. Each
capture has a matching `-observation.json` dump. The matched stale-fixture
before/after captures above also pass visual inspection. The native full
workspace suite remains separate from this focused verification.
These captures establish native Linux software rendering only; no native Windows
interaction or platform rendering is claimed.
