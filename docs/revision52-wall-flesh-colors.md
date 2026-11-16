# Wall faces and stale-flesh backing

## Request and implementation

The designer requested: “Color # differently for walls and for stale flesh (the area behind the walls)”. Both are canonical `wall` tiles, not distinct simulation materials. Previously they shared the same Flesh wall ink, although the renderer already identified exposed wall edges from known cardinal neighbors.

The same existing edge predicate now selects the foreground of an ordinary wall's `#`: the exposed face retains authored bone ink `#b4a39a`, while solid backing uses the existing muted earth/flesh ink `#bb7c72`. A known cardinal neighbor other than wall, chasm or pit supplies an exposed edge. Diagonal, out-of-map and unknown neighbors do not. Remembered cells do not borrow currently observed neighbors' live identities, matching the established edge rule.

This is an input-palette distinction, before the existing light, variation, material, global, fog and opacity stages. It does not bypass display illumination or readability. Explicitly edited wall colors remain authoritative, fixtures and overlays retain their existing treatment, and the frozen v48 presentation adapter is unchanged. The same adaptation is exercised by the test-only JSON material oracle.

Remembered terrain retains the existing neutral-gray fog policy. Native measurements confirm that this scene's exposed faces remain gray81 and backing remains gray70 on the same gray21 background. Both classes remain distinguishable without altering the gray70 wall readability floor, adding a detached marker, or changing repaint fingerprints. Existing fingerprints already include the foreground color.

## Boundaries

The classifier accepts only detached `TerrainObservation` and `ObservedCell` values. It cannot inspect the actual floor, unknown terrain, hidden actors, current hidden illumination, gameplay RNG or simulation state. Unknown cells remain absent and their backgrounds remain black. Terrain labels and glyphs stay `wall` and `#`; no movement, damage, sight, normal/haste behavior, save fields, replay or time-travel rules change. Restored states and repeated/retained frames use the same observation path.

## Verification

Focused deterministic tests cover the existing cardinal-edge identity; unknown and diagonal neighbors; grid bounds; current-versus-remembered neighbor filtering; fixtures; explicit color/glyph/material/global/fog controls; legacy/compiled JSON-reference parity; actual LOS-hidden terrain changes; save-state immutability and deserialization; native color/contrast evidence; unknown native pixels; and full/retained/tiled pixel equality through illumination, visibility and observed-geometry changes.

The integrated native desktop test binary from compile18 passed all five new tests, both existing compiled-ground/JSON native-pixel oracles, eleven illumination tests, five existing Flesh palette tests, the ordinary remembered-geometry test and the frozen v48 avatar-scene test: 25 passes in total, with one pre-existing isolated SDL/GPU calibration test explicitly ignored. The final compile20 binary reran the strengthened five-test wall suite: **5 passed, 0 failed, 0 ignored in 2.42 seconds**. Exact gray81/70 assertions, black unknown-cell pixels, and full-frame equality after hidden terrain/light mutations all passed. No source changes were required after that run.

Measured final paint colors in the controlled 79×29 scene:

| Knowledge / illumination | Exposed `#` | Backing `#` | Shared background | Face / backing contrast |
|---|---|---|---|---|
| Visible / -100 | 91,91,87 | 93,90,90 | 3,3,3 | 3.024 / 3.021 |
| Visible / 0 | 128,112,104 | 148,103,97 | 62,52,53 | 2.538 / 2.502 |
| Visible / +100 | 250,219,203 | 251,175,165 | 211,177,180 | 1.498 / 1.099 |
| Remembered / hidden -100, 0 or +100 | 81,81,81 | 70,70,70 | 21,21,21 | 2.301 / 1.935 |

The existing illumination ramp still desaturates near darkness and compresses contrast under extreme positive illumination; the change does not bypass those policies to force a flat color or new contrast floor. At +100, the exposed wall is lighter bone while backing becomes lower-contrast pink. Native visible0, visible100 and remembered0 screenshots were visually inspected initially. The final compile20 actual-observation, visible0 and remembered0 captures were also inspected. The three remembered frames have identical RGBA CRCs despite their hidden illumination inputs; the six controlled compile20 captures and measured color/contrast records are unchanged from compile18.

Evidence is in [verification/r52-wall-flesh-colors](../verification/r52-wall-flesh-colors/), with final receipts under `compile20/`: seven direct 1280×800 native PNGs, foreground/background/contrast/luminance records, tile and frame RGBA CRCs, source/binary hashes and individual test logs. Full/retained/tiled tests compare 28 exact frame pairs through seven material/visibility/geometry states and two presentation times, plus one full-frame privacy comparison. Native software-renderer captures do not constitute a Windows runtime or hardware-GPU pass. The integration owner runs the final unfiltered workspace and Windows build gates.

## Complexity

Let N be observed grid cells and W the subset of known ordinary walls using default foreground. Each terrain-frame rebuild adds at most four bounded detached neighbor probes per W cell: O(W) additional time and O(1) extra scratch per cell, with no allocation, persisted classification or cross-frame cache. The existing terrain preparation remains O(N) plus its existing profile/light/material work. No visibility, pathfinding, simulation or native glyph raster work is added by classification. Native rendering still depends on covered pixels, existing glyph work and glyph-cache misses. This is asymptotic analysis, not a new measured performance claim.
