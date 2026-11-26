Revision 51 stair exclusion: see [revision51-stair-exclusion.md](revision51-stair-exclusion.md) for shared movement/spawn gates, adjacent/deferred arrivals, finite Mercuria flood costs, legacy behavior and verification scope.

Revision 51 player fixed attack base: see [revision51-player-base-damage.md](revision51-player-base-damage.md) for constant arithmetic per attack/preview, inherited live-stat lookup costs, copy ownership and deterministic validation scope.

Revision 51 blocked enemy pursuit: see [revision51-enemy-reroute.md](revision51-enemy-reroute.md) for occupancy/terrain helper costs, weighted guaranteed-turn routing, deterministic ties, bounded graph memory, target-knowledge preservation, and verification scope.

Revision 50 Greatsword-equivalent offense: see [revision50-player-offense.md](revision50-player-offense.md) for live stat lookup, constant arithmetic, Mini-Me/preview changes, unchanged enemy birth and generation admission, the offline body-only sensitivity recurrence, and the bounded native encounter diagnostic. Its native results and remaining campaign/release limits are recorded there.

Native Sphinx and Ruin Pulse: see [sphinx-ruin-coverage.md](sphinx-ruin-coverage.md) for stable-ID/LOS/body-packet complexity and transitive death/visibility costs.

Revision 50 Waking Nightmares: see [revision50-nightmares-coverage.md](revision50-nightmares-coverage.md) for independent serialized decoy admission, visibility and registry costs, normal-turn recharge, and stable actor removal.

Revision 50 stats-only Mini-Me: see [mini-me-coverage.md](mini-me-coverage.md) for frozen effective numeric stats, no copied abilities/items/reserves, one-actor ID admission and placement/COW costs. [Actor-local targeting visibility](revision50-actor-visibility.md) documents generic independent-owner sensing and cache costs; it is no longer a Mini-Me ability path. Stats-only construction and rewritten synthetic-owner fixtures pass the focused native core suite; see [the current native report](revision50-native-verification.md) for aggregate verification.

Desktop ability command projection: see [ability-command-projection.md](ability-command-projection.md) for equipped craft controls, active-target deactivation, adaptive details, and aura/LOS costs.


### Main menu and Continue (October 2026)
Six fixed menu rows share hover/click bounds; hit testing, keyboard selection, and resume availability are O(1) time/extra memory per input/frame. Continue moves ownership of an already validated startup save, O(1), or returns to the active engine without reloading older disk progress. An active terminal run disables Continue even if an older live save existed. Missing/invalid/terminal startup saves disable it. Startup reads at most 32 MiB once using the existing full session decode and supported-mechanics validation; its transitive parsing/engine validation costs are unchanged (not O(1)). Let S be serialized save bytes and E decoded engine memory: startup adds O(S + E) transient memory and retains at most one E-sized candidate until activation or replacement by a new/imported run. Nothing is read or decoded on hover/render; the candidate is released when an active run supersedes it. Six menu rectangles/labels are constant-count logical primitives; actual rasterization depends on drawable pixel area and glyph work as documented by the rendering subsystem. Tests cover six-row bounds/gaps, no-save/terminal gating, exact save transfer with provenance, and preserving live progress over stale saves. Native Windows pointer verification remains a separate platform check.

Native drawable-resolution presentation: see [native-resolution-rendering.md](native-resolution-rendering.md) for framebuffer, font/mask cache, upload and input mapping costs.

Revision 50 corpse/blood/dialog visual follow-up: see [scope, native captures and complexity](revision50-visual-follow-up.md) for the unchanged admitted marker paths, 84% shared modal transform, renderer parity and fog/preference regressions.

### Ability heading total (October 2026)
Observation captures the length of the existing ability projection once, before the display list is sliced for scrolling. The heading reads this stored total rather than the remaining row count. This adds O(1) time and one `usize` of memory per view refresh; it adds no catalog scan and does not change the existing projection or scroll complexity. Regression `observe::tests::ability_total_survives_scrolling` checks the total after slicing and clearing displayed rows, retaining acquisition order.

Embedded bestiary artwork, icon, and Windows manifest: see [native-embedded-art.md](native-embedded-art.md) for build/index/decode/cache memory bounds and distribution provenance.

Fixed-page bestiary: see [native-manual-layout.md](native-manual-layout.md) for complete-content wrapping, deterministic depth/creature composition, aspect-preserving cutout processing and native rendering costs.

Expanded native creature depth windows: see [native-monster-depth-windows.md](native-monster-depth-windows.md) for revision-aware ordinary/Ichor eligibility, bounded recipe cache construction and planner costs, deterministic samples, manual synchronization and backward replay compatibility.

### Native display modes (October 2026)
Windowed, borderless fullscreen, and exclusive fullscreen form a fixed three-value cycle. Selection, SDL enum conversion, actual-state reconciliation, and mode validation are O(1) time and extra memory per activation or startup. Persistence uses the existing bounded appearance-library transaction: for B serialized library bytes (at most the existing 2-million UTF-16-character budget), serialization, validation, and atomic write are O(B) time/extra memory, plus filesystem sync latency. No library write occurs per frame. Legacy fullscreen booleans load as windowed/borderless; explicit mode strings reject unknown values before state mutation. SDL/OS display switching and compositor reconfiguration are external driver-dependent operations, not claimed O(1) wall time; platform availability and visual switching require actual desktop checks. Tests cover all mode reloads, legacy migration, invalid-mode rejection, preset/motion writes preserving mode, and failed persistence retaining saved settings. Rendering cost is unchanged from the existing constant-count settings buttons and drawable-resolution rasterization.

### Recomposed title-page menu (October 2026)
The six shared hit regions are constant-count O(1) input work. Hand-cut title capitals use fixed continuous polygon contours and static distress cuts. Native even-odd scan conversion takes O(H×E log E + A) time and O(E) fixed scratch memory per glyph, where H is glyph pixel height, E is the bounded edge count (at most 32), and A is covered drawable pixels. Sorting intersections is included; there is no font lookup or title glyph cache miss. A four-offset ink keyline adds a fixed factor to inactive label painting. Text uses the existing native glyph cache (cold font rasterization, hot glyph tinting); the six fixed labels and sizes bound its entries. The unchanged original RGBA artwork is nearest-neighbor sampled into an aspect-preserving destination, O(I + A) time and O(I) transient memory for its existing clone, where I is source image bytes and A destination pixels. The menu fills the whole drawable flame background, including aspect-ratio wings, using the existing bounded flame field and native embers; framebuffer and upload remain O(W×H). No render or hover path reads saves or changes simulation. Bounds/corner/gap tests and custom letter coverage tests are deterministic; screenshots exercise actual renderer at 16:9 and 16:10. Platform fullscreen behavior must be verified separately.

Native path tints and drawable-resolution embers: see [path-ember-revision.md](path-ember-revision.md) for observation-safe compositing, hash-set assumptions, and bounded particle rasterization.

### Gameplay hierarchy and remembered-map contrast (October 20, 2026)
The native painter consumes only the already-redacted observation cells. Let C be painted cells, G total glyph characters, L characters in the existing last-30-message chronicle, A the chronicle's native pixel area, R visible ability rows (bounded by the existing fixed allocation), and N the characters in a row's label/metadata. The remembered/visible terrain and creature contrast treatment adds O(C) time and O(1) scratch space per frame; `distinguish` performs a fixed bounded contrast adjustment, never visibility, world lookup, light simulation, or pathfinding. It does not manufacture cells for unknown positions. Custom fog/global appearance edits bypass native contrast-floor treatment. Player backing is one bounded tile rectangle. Glyph rasterization/cache misses and framebuffer upload retain the native drawable-resolution costs documented above.

Chronicle measured word/long-token wrapping takes O(L²) worst-case font-metric work and O(L) retained/transient text per frame, including candidate string creation and rescans; font metrics use existing font objects. It displays four complete default line tracks plus the older half-track in a fixed 108-logical-pixel clip. The continuous per-pixel fade adds O(A) time and O(1) scratch memory, without copying the framebuffer. It never sorts or changes messages. Ability name and metadata ellipsis uses O(R × N log N) font-metric work plus O(N) scratch per fitted field; fixed two-line rows reserve name width separately from a constant cost column. The same existing font-cache cold-rasterization costs apply. Toolbar focus performs ten fixed-region checks/paints, O(1) logical work plus O(covered native pixels) raster work, with no input dispatch or simulation mutation.

Deterministic regressions verify unknown black space, remembered geometry and restrained water color, identical frames after hidden terrain/actor edits, long-token wrap bounds, monotonic continuous fade, full `Unyielding Assault` at default geometry, fixed map/navigation pixels after large changes in sidebar content, preserved path tint, and cosmetic frame stability. Opt-in `CORPSE_GAME_UI_REVIEW` exports native 1280×800 and 1920×1080 screenshots. Costs above are asymptotic analyses, not measured timings; use representative generated/explored floors and varying DPI/log lengths for profiling. OS/window/fullscreen behavior still requires platform checks.

### Basic display settings
`display_ui` opens a draft-only modal with four fixed hit regions and at most 256
validated resolution entries. Opening is O(min(n,256) log min(n,256)) time and
O(min(n,256)) extra memory for n provided modes (sort/dedup). Keys/clicks are O(1);
Apply is the sole request to the host for SDL/storage mutation, and Back discards
unapplied changes. Drawing uses a constant count of rect/text calls over the
1280x800 logical canvas: raster work is O(P + G) for painted pixels P and glyph
coverage G, with existing font/cache-miss behavior in the renderer. No filesystem
or native display queries run during drawing. Combined mode/resolution persistence
serializes the bounded appearance envelope and performs one atomic write: O(B)
time/extra bytes for serialized size B (2 MB UTF-16 cap), plus OS sync/rename cost.
Platform mode switching and compositor timings remain unmeasured; validate on real
windowed, borderless and exclusive-fullscreen drivers separately.


### Animated bestiary scenery (October 20, 2026)
See [native-manual-layout.md](native-manual-layout.md#creature-specific-animated-scenery-october-20-2026) for explicit secondary creature profiles, five authored depth-environment stages, the 48,320-sample atmospheric field, bounded polygon scan conversion and structural/creature geometry, native raster/overdraw costs, warm-frame frequency, reduced-motion behavior, deterministic tests and reproducible QA controls. This is presentation-only; simulation, save and replay complexity are unchanged.

October 20 clean-art correction: bestiary decorative interaction raster pass removed entirely. No decal geometry allocation or raster work remains; static art/text rendering bounds remain documented in native-manual-layout.md.

### Bestiary setting-name emphasis (October 23, 2026)
Natural History and other bestiary body sections canonicalize the setting name
“the Flesh” and use the existing bundled ExtraBold face for its full phrase,
including when it wraps across lines. Anatomical flesh is not matched. Per
selected-page layout candidate, ASCII folding, whitespace normalization, word
wrapping, and a monotone range/line merge use O(C) time and O(C) temporary memory
for C authored characters. There are at most five font-size candidates (16–20).
Equal-byte-length capitalization does not shift text; the two bundled monospace
faces have equal per-glyph advances for the phrase, verified at all five sizes.
Painting adds O(R + L) prose calls for R emphasis ranges and L lines, with O(G)
raster work over glyph coverage G, including existing uncached fontdue raster
costs. No extra fonts, persistent caches, animation, simulation, or save changes.
Validate with the manual module tests, including wrapped spans and rendered-pixel
comparison; native frame timing is unmeasured.

### Native Flesh materials and cosmetic blood (October 25, 2026)
See [native-flesh-visuals.md](native-flesh-visuals.md) for observation-only palette
substitution, bounded organic color ridges, water/Ichor distinction, revision31
cosmetic footprint costs including first-layer allocation and Arc copy-on-write,
legacy differential separation, save/replay boundaries and renderer validation.

### Continuous bestiary visual-depth colors (October 25, 2026)
The solid page color uses the selected entry's first-appearance visual depth,
independently of combat calibration reference floors. This preserves monotonic
darkening through the first-appearance roster. The established floor1 parchment, floor25 red and floor40
black anchors are interpolated continuously; no spatial gradient or new palette
anchors were introduced. The two-segment interpolation and contrast choice are
O(1) time/memory per page paint, using the already selected depth; no gameplay
state is read. Art decoding/cache misses,
full-coverage ink splash, typography, ordering and full-frame O(A) raster costs
are unchanged. Tests verify anchor endpoints, intermediate half-floors, continuity
at the central anchor, solid page pixels and unchanged roster/order/first-depth.
Native `--render-test --manual-entry` captures use the same live visual-depth
path unless `--manual-depth` explicitly overrides it. This O(1) dispatch adds no
allocation or rendering pass; all existing page raster and asset-cache costs
remain unchanged. Fractional explicit overrides keep continuous interpolation;
depth-dependent stats use the lower whole floor. Regressions verify pixel parity
with the live page, explicit override behavior, unchanged selection/roster, and
no backward color steps over all 79 pages.

Persistent native floor-one entrance (revision 31): see
[native-floor-one-entrance.md](native-floor-one-entrance.md) for the one-time,
RNG-free safe connector, metadata/fixture expansion, bounded stable Dijkstra,
constant-sized persistent identity, legacy replay boundary and verification.

### Seeded floor playtester (October 25, 2026)
See [native-floor-playtester.md](native-floor-playtester.md#complexity-and-bounds) for actual prior-floor reward derivation, generator/offer/equipment transitive costs, bounded asynchronous startup/cancellation, seven-row input, serialized provenance, replay regeneration and score exclusion. The seventh title-menu row uses the same fixed hit-region/raster rules as the six older rows; old six-row descriptions above describe the prior revision.


Provisional revision31 physical encounter scale: see [physical-encounter-balance-v31.md](physical-encounter-balance-v31.md#complexity) for the 48-record new-actor numeric overlay, explicit Pale and Ichor construction paths, independent reference-depth lookup, unchanged population policy, offline joint-cohort costs, and bounded native dispatch/observation costs. The linked entry separates additional harness work from existing validation, pathfinding, visibility and environment processing; developer fixtures are not embedded in the shipping binary.

### Native finale terminal and archive presentation (October 25, 2026)
See [native-finale-presentation.md](native-finale-presentation.md#complexity-and-bounds)
for optional ending storage, legacy shape preservation, summary/archive string
costs, bounded validation, unchanged native glyph/frame costs, and test scope.

### Flesh Administrator and staircase escape collapse (October 25, 2026)
See [native-flesh-finale.md](native-flesh-finale.md#complexity-and-bounds) for the
separate budget-neutral boss placement, finite two-turn eight-neighbor frontier,
whole-world reached-floor propagation, maintained-link cleanup costs, COW temporal
storage, provenance-based fall hooks and generator/AI/rendering transitive costs.

### Content- and silhouette-responsive bestiary composition (October 25, 2026)
See [native-manual-layout.md](native-manual-layout.md#complexity-and-bounds) for
cached O(P) selected-art trimming, bounded text-fit candidates with O(C + L)
working storage, actual O(N + C + L + A + F) warm paint costs, fixed no-scroll
geometry, and lossless-content / occupied-pixel / protected-region assertions.
No mechanics, asset catalog, save format, controls or gameplay RNG changed.

### Reviewed creature alpha assets (October 26, 2026)
See [creature-alpha.md](creature-alpha.md#complexity-and-bounds) for offline graph-cut, local edge-reconstruction and review costs, and cold PNG decode/cache bounds. Runtime perimeter matting has been removed; unreviewed assets are explicitly frozen legacy references, not newly cleaned.

### Revision 32 monster balance and creature summon clock (October 26, 2026)

The revisioned physical overlay scans T fixed catalog rows once per newly constructed
actor: O(T) time, O(1) per-actor extra scratch; parsed catalogs retain O(T) data through
OnceLock. No overlay runs during load or each turn. The selection makes no RNG call
and does not alter rewards. Version validation adds one fixed enum case.

A Pale birth readiness check reads one condition and compares the global serialized
world tick, O(1) expected condition-map lookup and O(1) extra actor storage. Successful
birth writes one timestamp after its existing one/two-child construction; blocked
casts write none. No per-turn, haste, offscreen or snapshot-specific timer loop was
added. The existing eight-neighbor occupancy scan remains O(A) in floor actor count A
(8 bounded neighbors, each scanning A), plus ordinary actor construction O(T+K) for
fixed tuning/trait catalog lengths T/K and existing events/visibility work. Snapshot
and save costs retain their existing O(S) state costs and copy-on-write actor blocks;
one numeric condition adds constant serialized bytes per summoner. The absolute
clock moves back with both temporal restore routes, avoiding separate elapsed-time
migration. Legacy saves with no key are ready by default. These are asymptotic
analyses, not measured wall-time claims; cooldown regressions and native trial drivers
provide reproducible verification.

## Revision32 flavor placement

See [flavor-placement-v32.md](flavor-placement-v32.md#complexity) for explicit57-row lookup,
55-leader recipe/cache bounds,45-definition setup filtering, fixed40-floor corpse mapping,
and build-time79-entry manual synchronization. Revision wrappers preserve legacy geometry
and generation complexity and introduce no per-turn loop, global switch or gameplay RNG.

## Full-generation player heart audit (developer analysis)

See [player-heart-progression-audit-v32.md](player-heart-progression-audit-v32.md#complexity)
for the actual-generator census, four native consumption tracks, paired-seed summaries,
and bounded fish/full-energy summon sensitivity. This is an offline example/script
pipeline; no generation, heart reward, or player progression gameplay cost is changed.

The optional `combat_audit` observer is disabled during normal gameplay. A disabled
record costs one thread-local option check; an enabled command-scoped batch appends
one already-mitigated packet in amortized O(1), retaining O(P) packets for that
command. It consumes no RNG, never enters saved state, and no gameplay rule reads
it. The native balance harness uses its pre-cap and HP-capped amounts because some
real attacks deliberately have no numeric text log. This is analysis instrumentation,
not a second damage implementation. Its parity test compares complete simulation
state with observation enabled/disabled. Dynamic summon inspection adds a fixed
number of string/condition reads (O(L) output text), with no world lookup or mutation.

Native whole-roster lifetime/gas-escape/support/brood analysis: see [monster-balance-native-audit.md](monster-balance-native-audit.md).

Revision32 birth-only gas-volume metadata adds up to three numeric conditions per
applicable actor. New-actor lookup uses the same O(T) overlay scan; each passive
emission or successful gas spell adds one O(1)-expected condition lookup. It does
not change diffusion's per-grid work, gas damage calculation, actor iteration,
spell costs, or RNG draws. Fixed brood lookup scans six rows, O(6), once per birth;
the actor stores resolved values so later inspection/load/time travel never retunes
it or consults its current depth. The six-row table has O(6) retained data. These
counts are authoring bounds, not measured performance.

### Revision-33 water blood, hallucinations, Disoriented, and root escape
See [native-environment-hooks-r33.md](native-environment-hooks-r33.md) for exact
revision gates, source/diffusion constants, normal-versus-haste/offscreen clocks,
optional save/provenance layers, binding-link scope, O(N + K) bounded transport,
O(A) status clocks, O(P) root severing, and explicit allocation/COW/rendering costs.

### Revision33 Witch summons, creature observation and population
See [witch-creatures-v33.md](witch-creatures-v33.md) for mechanics, constants,
verification and explicit numeric-scope limits. A Witch summon scans A floor
actors into an occupied-cell hash set (expected O(A) time/memory), then tests at
most8 adjacent cells and two fixed templates. Existing horn permission/silence
checks can scan A actors and perform O(R) rays, giving O(A*R + A + T + S) worst
case including metadata tuning scan T and copied child state S; no pathfinding is
introduced. The cooldown lookup is O(1) expected; unsuccessful preflight does not
draw RNG or mutate Energy. Persistent new conditions add O(1) per actor.

Revision33 roster caches are separate from revisions29–32. For K species, maximum
encounter size M=5 and C emitted recipes, the existing recipe builder remains
O(K²*M² + C*K*M) worst case including stable dedup keys and introduction scans,
with O(C*M) cache memory per depth/habitat (80 bounded caches). The revision adds
only two leaders (K=57 rather than55); exact-heart feasibility retains the existing
bounded-budget solver and is not included in a false O(K) population claim.

Appearance projection is called only after existing perception admission. For A
floor actors, B<=2 observing bodies, ray length R, actor-copy size S, seed bytes L,
and fixed tuning entries T, one changed presentation costs
O(A + B*R + S + L + T) worst case and O(S) scratch. Whole-frame work includes this
for V observed actors, plus the existing terrain/perception work; worst-case
O(V*A) Half Here searches are not hidden. No render-driven simulation updates or
RNG calls occur. Inspector projection additionally clones the Run's non-floor
metadata N and COW-detaches one actor block, O(N + A*S) time/memory; floor cells
remain shared. PNG decode, glyph-cache misses, raster work and uploads keep their
existing documented costs; two extra plates add decoded image memory only when
selected. Water tint is constant work per admitted water cell with O(1) scratch;
no hidden floor layer is read. These are asymptotic estimates, not measured frame
timings; profile generated crowded floors, Half Here, repeated hovers and cold
bestiary image selections before claiming performance improvement.

### Revision-33 creature feeding and bloody-wall ecology
See [native-creature-feeding-r33.md](native-creature-feeding-r33.md#complexity)
for normal-turn hunger/meal clocks, two bounded safe-food BFS fields (including
hazard-helper/cache-miss costs), exact involved offscreen turns, COW/save costs,
deterministic O(N) wall overlays and observation-gated rendering. Meal timing
and resources are separately specified and regression-tested.

### Revision-33 deeper Zillennia ground contact
See [native-zillennia-ground-r33.md](native-zillennia-ground-r33.md#complexity)
for arrival O(A + F + G) expected scans, initial placement O(A + F + G + Z),
O(Z) scratch, explicit changed-block Arc COW costs, canonical save/time-travel
ownership and observation-only knowledge updates. No radius scan, damage loop,
per-frame mutation or new persistent mechanic state is introduced.

## r33 localized Nest/pocket biomes and Royal Honey

- `pocket_biomes::decorate`: once per generation candidate, with independent seed stream; N floor cells <=3600, R existing rooms, C sum of room-cell metadata, P protected fixture/footprint cells. Safe-mask preparation and gas-component labeling O(N+P), O(N+P) scratch; hazard-source component exclusions use bounded per-setup origins and existing vent/fire lists. At most three regions, each tries at most R anchors; each anchor BFS and room scoring/sorting costs O(N+C+R log R), plus O(N log N) bounded ordered-set assembly/membership (not merely O(N)). Worst case O(P+N+3R*(N log N+C+R log R)); extra O(N+C+R+P). Existing generation remains bounded to32 candidates. The region never performs nested pathfinding per monster; at most96 existing water-link cells become floor bridges, or the explicit all-water fallback raises <=160 water cells across compact existing footprints+links, only adding ground connectivity. The fallback tries at most one additional bounded region plan. Protected generation-cell construction retains its existing bounded setup/fixture assumptions.
- `pocket_biomes::populate`: once per generated floor; O(A+G+B*(A+N)) expected time/O(A+G+N) scratch with hash occupancy, A actors, G ground entries, B<=3 pockets. Queen+6 residents+1 Honey are bounded additions. Matching ordinary actors are relocated, not re-created. Creature construction uses the existing bounded catalog, trait and health initialization costs.
- `saidribee_queen::summon`: attempted from existing AI action dispatch; O(A+8*H) time and O(A) occupancy scratch, H existing per-cell creature-hazard helper cost, plus bounded actor construction/event emission. At most1 unit per successfulcast;15energy and10worldturn cooldown. No RNG on ordinary rejected casts. Observer visibility/event helpers retain existing LOS/perception costs.
- `royal_honey::eat`: bounded effective-stat/equipment traversal plus existing health-sync/half-here helpers; inventory lookup/removal O(I), direct underfoot lookup/removal O(G). Resource refill is O(1) after effective maxima resolve. No new clocks, caches, or per-turn population scans.
- Observation: per known cell, at most3 sorted-cell binary searches plus optional flower binary search, O(B log N); output only when terrain knowledge permits it. Renderer uses constant-time palette/glyph selection, then unchanged native lighting/font rendering; native GPU/font cache misses are not measured here.
- Timings are not asymptotic guarantees. Targeted optimized regression command: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_OPT_LEVEL=1 bash tools/cargo.sh test -p corpse-core --test nest_pockets_v33`; end-to-end generation profiling must include rejected quality candidates. See `docs/nest-and-pocket-biomes-r33.md` for tuning and scope.

## Revision33 Borge event

See [Borge event and complexity](borge-event-r33.md). Central placement is
O(V×S) plus existing generation repairs; first selection O(A×(F+B+H)), exact-heart
provenance O(F), offering O(I), and one-use local targeting O(I+A+B) **plus** the
existing full death/placement/boss lifecycle costs. F≤40, S=2; no additional RNG
or external quest cache. The linked entry includes cold catalog, renderer,
serialization, native costs and profiling instructions.

## Revision 33 debug floor reconstruction

`debug_floor` uses the actual normal startup/generation pipeline for D=max(10,target)
≤40 floors. Its geometry, placement, spatial/pathfinding feasibility, actor constructors
and cache misses have the existing generation subsystem costs; this wrapper does not
replace them with cheap expected values. Let H be the actual initially generated
heart-bearing actors plus generated ground hearts on preceding floors, G the prior
normal ground items, C the actual carried loot, A the authored ability catalog, and
L retained log/resource records. Enumeration and source removal are O(H+G+C), apart
from Sphinx reward-cell membership (a fixed four cells). Pending hearts and recovered
stash require O(H+G+C) extra storage; normal inventory stays at a twenty-slot selection.

Heart consumption calls the real inventory/progression paths, including equipment,
health, adaptive-heart tracking and log/resource bookkeeping. At each earned level,
execution stops for the real user choice; offers retain their existing O(A²) bound.
No combat/navigation/event simulation occurs. Normal transactional command dispatch
also clones debug metadata, O(H+G+C+L) worst case, in addition to its existing world
Arc/snapshot/validation costs. Stash mutation occurs only on explicit debug actions:
linear ID/capacity lookup O(I+G+C), I≤20; transactional equip additionally clones the
normal run metadata and invokes existing equipment reconciliation. The renderer
visits/allocates references to all stash/inventory entries per dirty redraw, O(G+C+I),
but paints at most 26 rows. Native font/raster work is bounded by those visible rows
and existing renderer clipping. Save serialization includes stash/remaining hearts;
all debug autosave/high-score writes are disabled. Timings are unmeasured; reproduce
with `cargo test -p corpse-core --test debug_floor_v33` and the documented CLI workflow.

Revision 51 DEBUG reconstruction additionally normalizes fungal links once after
removing preceding-floor sources. With D<=40 floors, A retained actors, E inspected
ability entries, V total cells, K links, P saved path entries and R<=11 canonical
path cells per floor, this adds expected O(D+A+E+V+P+K*R) time and peak
O(max_floor(A+V+K+R)) scratch. Only floors carrying root state pay normalization;
existing floor COW copies remain additive. Worst-case hash collisions retain the
fungus subsystem's per-floor O(A²+A*K+K²) costs. No RNG is drawn, and the helper is
O(1) before revision 51. See [floor 40 regression](floor40-debug-investigation.md)
for save/resume and dirty-floor scheduling evidence. This is setup-time cleanup,
not an extra per-turn pass.

## Revision33 flavor Accuracy/Dodge specialization and analysis

See [accuracy-dodge-r33.md](accuracy-dodge-r33.md). Immutable natural K=17 and
Witch-born B=2 profile scans run only at new-actor construction, after their
separate body profiles: O(K) / O(B) time, O(1) mutation memory after O(K+B)
one-time catalog parsing/cache. No RNG, per-turn/render lookup or loaded/grown
actor rescaling. Analysis retains full native dispatch/pathfinding/environment
cost and explicit256-command horizons; the doc separates those costs, joint
profile sorting/storage, and constant-work maximum-packet observation. Focused
35-test and bounded native-matrix receipts are not a full-run complexity or
survival-performance claim.

DEBUG live-stash temporal reconciliation uses a serialized loadout revision:
O(1) comparison detects direct zero-turn take/put/equip/entry edits. On an actual
revision change, dispatch/Revert refresh the closing snapshot; explicit `save(&self)`
uses the matching effective closing snapshot without mutating the engine. Snapshot
metadata clone and existing history inverse/serialization costs still apply; they
are not constant-time operations (five completed frames remain the bounded ring).
Pending haste retains the ordinary opening-snapshot path. This prevents stale
inventory/stash array diffs from losing/duplicating loot during Revert/save-load;
normal runs and legacy playtest v1 bypass this debug-only reconciliation.

### Revision33 actor-aware message identity
See the combat-message correction in [witch-creatures-v33.md](witch-creatures-v33.md).
Core narration and desktop projection share the same pure appearance selector;
M name projections add O(M*(A+B*R+L)) work, B<=2 bodies, plus existing visibility
helper costs. Names borrow stable strings; lethal packets retain only their
short display label. No persistent identity/state/RNG mutation or global log
replacement is used; true death-source provenance and rewards are preserved.

### Revision 34 — Godbeasts and native planes

Let V be floor cells, A actors, I ground/inventory items, F generated floors, P
player abilities/statuses, M cloned Run metadata, and R a ray's traversed cells.
All floors are bounded by the existing 40-floor world. See [release-r34.md](release-r34.md)
for rules and verification boundaries; these are asymptotic estimates, not timing
claims.

Dream activation creates one endpoint O(P), plus actor scans O(A). Each floor's
first dream access copies its terrain/environment O(V); later plane swaps exchange
Arc references after O(F) lookup. Arc copy-on-write on first mutation costs O(V)
and is not constant-time. Both persistent plane layers add O(FV) storage; actor,
inventory and reward tables remain single authoritative tables, not cloned worlds.
Environment processing runs at most twice, retaining each original propagation,
LOS and chain-effect complexity, plus O(A) contact filtering. Shared-owner resource
synchronization is O(A+P); active endpoint and actor-target gates cost existing
owner lookup O(A), not a new map. Rays remain O(R*A) worst case with actor scanning,
plus native-floor lookup O(F). Per-plane visibility retains original origin/LOS
costs; these gates do not claim constant-time whole observations. Save validation
adds O(FV) nested-floor validation/serialization, plus ordinary save costs.

Nightmare generation is O(V+A+I+K²V), K<=7 authored placements, O(V+A+I) scratch.
Four actors persist per floor and are never recreated on observation or entry.
Epidite normal-turn age increments O(1); a pulse scans A+I exposed candidates with
LOS O(R) each plus existing damage/death chains. Visited inactive floors retain
existing offscreen simulation costs. Blessing one heart costs its bounded stats
record size and no extra RNG. Ahvamitt destination selection is O(V*(A+B))+O(V)
BFS, B existing blocking checks, O(V) scratch; one deterministic RNG draw occurs
only on successful selection. Third reward uses ordinary ground placement and
actor removal O(A), including existing generation callbacks. Mask lookup O(I),
cooldown O(1), never frame RNG.

Desktop active dream observation clones Run metadata O(M), sharing Arc floor
cells; inactive views borrow. Actor layering O(A log A) time/O(A) scratch. Palette
application O(C+G+U) for visible cells, glyphs and bounded underlays, plus native
font raster/cache and framebuffer upload costs. At most two per-plane hazard
contexts are built per terrain observation, not per cell. Epidite tower lookup
O(A+F+C), fixed two-digit timer; Teleport row O(I) mask scan/O(1) cooldown. No
rendering writes simulation or advances the world clock.

Native-plane terrain observation stores O(V) plane tags and O(P) viewpoint tags,
prepares at most two hazard indexes O(H), and selects the native floor in the
existing O(V·P·L) visibility scan; it removes the duplicate union-chance scan.
Here H is indexed hazard entries, P observing endpoints, and L includes lighting
and ray work. Native memory admission caches P origins and A sensed positions:
O(A·P·R + V·(A + P·L)) worst case, O(A+P+V) scratch, R ray length. These visibility
P/L definitions are local to this paragraph, not the ability-count P above.

## Revision 35 portal topology

Let V be floor cells (at most 3600), A actors, P portal pairs (validated at most 7; generated default 1), and R a bounded ray length (at most 2V). Floor metadata is O(P). Pure endpoint lookup is O(P); shortest portal-distance queries use at most 2P+2 nodes, O(P²) time/O(P) scratch. Optical paths add ray obstruction checks, O(P²R). Projectile translated-aim search is O(P²R), with actor interception O(RA); it retains a bounded visited-pair set. Visibility calls these per observer/target; no constant-time visibility or exhaustive performance claim is made.

Portal-aware distance fields retain O(V+E) BFS or O((V+E) log V) hazard Dijkstra, E bounded by local grid degree plus portal transitions, and O(V) extra lookup/scratch. See `portals-generation-r35.md` for placement and articulation costs. Offscreen/feeding/returning-ally movement shares these transitions rather than maintaining a second teleport algorithm.

The directed fire cone scans V targets only when portals are present, with O(V*(P²R+RA)) geometry/actor-query work plus existing ignition/damage costs, O(V+A+R+P) scratch. Legacy and no-portal local cone traversal remains bounded by its prior local square. Consumer range checks gain O(P²); alias occupancy checks gain O(P) per candidate actor.

Desktop terrain overlays are O(VP); observed actor aliases O(AP). Route graph setup adds O(P) link indexing to the existing state graph. Knowledge-filtered projectile previews require O(V+A+P²R) work/scratch including the known-floor snapshot. Glyph styling is renderer-only and consumes no gameplay RNG. These are algorithmic bounds, not measured frame-time guarantees; profile multi-pair floors before increasing density.

Portal-aware Witch/Queen/caster-retreat placement checks scan A occupied aliases at O(AP) per bounded eight candidates instead of a constant-time occupied-cell hash lookup. Deterministic Boolean membership never consumes RNG or depends on set iteration order.

## Revision 36 non-Euclidean generation

See [the generation contract and detailed bounds](non-euclidean-generation-r36.md).
For floor cells `V`, directed proof edges `E≤8V`, pocket cells `K≤48`, existing
portal pairs `P≤7`, protected/fixture footprint entries `H`, initial actors `A`
and ground entries `I`, installation uses at most `S=48` CA shapes and `R=64`
assignment attempts per shape. Total worst-case work is
`O(V+H+A+I + S*(11² + R*(K² log K+V+E+P)))`; extra memory is
`O(V+E+H+A+I+K+P)`. The protected-generation helper's footprint expansion is
included in `H`, rather than assumed constant. Each fixed three-round 11×11 CA
is `O(11²)`; each ordered assignment is `O(K² log K)`; the forward/reverse
directed movement proof, including structural validation, is
`O(V+E+K log K+P)`. It includes both stochastic branches, existing portals and
all formerly reachable upstairs-component cells. Installation runs once after
floor population, with its own deterministic stream and no per-frame work.
These bounds are not measured timing claims. `install_with_report` returns
nonserialized diagnostic counts from the identical algorithm for repeatable
QA; it does not add draws, mutate the accepted distribution or affect gameplay.

### Revision 36 desktop observations and Epidite countdown

Pocket terrain decoration scans C detached cells against G groups, O(CG), with
G bounded by the 48-cell pocket limit. Known alias construction retains one
actor/item identity; per-input knowledge preparation includes O(V+K+A) floor,
pocket and actor data plus the existing visibility/ray costs. Route construction
still uses the existing phase-aware graph; a stochastic entry terminates the
route rather than expanding an unbounded tree of claimed future outcomes.
Known exits are hollow cyan marks and pocket terrain is a cyan inset, preserving
the underlying glyph and revision-35 portal style. Rendering requires the
ordinary font/glyph cache, rasterization and framebuffer upload costs; no RNG or
simulation steps are hidden in those native operations. The Epidite subscript
visibility test and countdown extraction are O(1) once the already-observed tower
has been found; this does not remove the existing observation/hover costs.
These are asymptotic bounds, not measured frame-time claims.

### Revision 36 physical topology queries and committed rays

Let V≤3600 floor cells, P≤7 portal pairs, K≤48 pocket cells, S≤32 pocket
source-to-destination edges, N=2+2P+2S≤80 route nodes (shared destinations may
repeat), A live actors, and R≤2V raster iterations. Alias/exits lookup is O(P+K)
time with at most three returned representations. Physical metric routing is
O(N²) time/O(N) scratch for constant-cost segment checks. Optical routing remains
portal-only, at most 16 nodes, and O(P²V) with raster LOS; Non-Euclidean groups do
not inflate the sight graph. Translated physical aim selection is O(N²V) worst
case including per-cell link lookups and O(N) candidate transforms. It does not
enumerate a tree of random futures. Each emitted ray crosses a group at most
once, and committed collision scans add O(VA(P+K)) work. Scratch is O(V+N).

Distance-field preparation indexes at most two arrivals per entry and reverse
adjacency in O(V+P+K), followed by O(V) BFS or O(V log V) hazard Dijkstra, with
O(V) scratch. A next-step decision examines at most 8×2 outcomes, requires every
outcome to remain reachable, ranks worst-branch hazard and optimistic progress,
and replans after the committed arrival. Caster/pull/vampire candidate ranking
multiplies their existing fixed eight-candidate work by at most two outcomes.
All aim/path/preview work is deterministic. Only an actually committed two-exit
transition consumes a fair gameplay RNG draw. These bounds include helper costs
but are not measured performance claims.

The revision-36 generation safety proof is also repeated with all portal
endpoints excluded together, so the changed movement graph cannot require
portal traversal. This is a constant factor of two, not a second per-cell
pathfinding loop; the generation bounds above are unchanged.


The linked fire cone retains at most V candidate rays, each with physical aim
and collision costs: O(V*(N²V+VA(P+K))) worst-case work plus ordinary ignition
and damage chains, O(V+A+N) scratch including emitted-aim deduplication.
Geometric/angle/local-sight candidate tests remain pure. Only an admitted
deduplicated ray launches committed physics, and ignition uses its actual
landing rather than pretending an uncertain preview chose an exit. The legacy
no-link local cone scan remains unchanged.


NPC possible-shot admission explores at most B=256 explicit branch prefixes
against the same fixed aim used at launch. Each pure trace includes ray-plan
selection, giving O(B*(N²V+VA(P+K))) worst-case time and O(BK+V+N) scratch, plus
existing local optical eligibility checks. The prefix search neither reads nor
advances future simulation RNG. Budget exhaustion declines that AI shot, rather
than pruning or changing the actual projectile lottery.

## Revision 37 terrain overlays and monster-driven sub-biomes

Let V be floor cells, A actors, I ground items, H Hellfire cells, P portal pairs,
K non-Euclidean/ordered groups, and C candidate habitat cells. Generation runs
once per populated floor and uses independent deterministic seed streams.
Habitat deletion proposals perform whole-floor before/after reachability checks,
O(V+E) each with E<=4V, with at most64 accepted removals and bounded proposal
lists; failed proposals also count toward the documented candidate bounds.
Each actor footprint and protected-cell construction must include its helper
scans rather than be described as constant-time per actor. Corn and link safety
add complete directed reachability checks, including all possible random returns
and a second proof with all portal endpoints avoided. The existing linked-route
planner's node count grows with P+K; its O((P+K)^2) graph search is not free merely
because generation is bounded. Concrete generation and turn timings, when
reported, are measurements rather than these asymptotic estimates.

A slip test is an O(1) overlay lookup plus existing flight/species queries, once
per admitted voluntary step, with at most one serialized gameplay random draw.
Path previews do not draw. Snow contact is O(1) plus the existing actor queries;
first allocation of its puddle layer costs O(V). The environment's snow contact
pass is O(A+V) per world tick and O(V) worst-case changed-cell scratch. Hellfire
initialization/advance adds O(V) scans and at most O(H) permanent-source scratch;
ordinary fire propagation costs remain additional. Offscreen dynamic detection
scans the Hellfire layer in O(V), selecting precise existing simulation phases.
Six optional live/remembered boolean arrays add O(V) state per floor, including
copy-on-write and serialization costs when a snapshot's floor is first mutated.
Desktop observation/rendering retains O(V) layer traversal plus existing optical
and link computations; cosmetic animation does not consume simulation RNG.

Detailed revision37 generation bounds, including all rejected proposals and transactional Corn candidates, are in [the generation contract](sub-biome-generation-r37.md).


## Revision 38 generation hardening

Direct tap reservations and geometry candidate windows are specified in [the geometry audit](generation-geometry-r38.md). Atomic full-floor retries, rollback state, exact-budget feasibility and error classification are specified in [the startup audit](generation-startup-r38.md). Bounds include at most eight complete floor attempts, each with at most 32 geometry candidates; no unbounded retry, silent required-content deletion or weakened safety validator is used. See [the release contract](generation-hardening-r38.md) for measured stress coverage and its limits.

Revision 39 Shard weapon actions: see [shard-ui-r39.md](shard-ui-r39.md) for revision gating, projection costs and targeted UI verification.

- Revision 39 Shard hit paths and dream-ground ownership: [contract and bounds](shard-weapon-r39.md).

### Compact r39 artwork distribution (October 30, 2026)
See [compact-runtime-art.md](compact-runtime-art.md) for the offline representation-only encoding and bounded WebP decode/cache costs. Canonical true-RGBA PNG export, silhouette and alpha are unchanged; only distribution RGB compression is approved. Simulation, nearest-neighbor drawing, and pixel compositing are unchanged.

The small-package follow-up in [compact-runtime-art.md](compact-runtime-art.md#smaller-package-build-and-ui-resources) also replaces four UI PNG resource representations with exact-RGBA lossless WebP. Constructor frequency/cache lifetime and drawing work are unchanged; compilation uses recorded size-optimization flags without simulation edits.


### Opt-in runtime timing diagnostics
See [timing-profiling.md](timing-profiling.md) for span collection overhead, sample/report complexity, deterministic equivalence checks and measurement limits; [desktop-timing-test.md](desktop-timing-test.md) describes projection/render/storage coverage. Feature-disabled builds compile spans away. This does not complete the paused universal function-complexity audit.

### First dirty-state/scheduling implementation batch

See [DIRTY-SCHEDULING-FIRST-PASS.md](DIRTY-SCHEDULING-FIRST-PASS.md) for the
implemented projected-lighting reuse, lazy path masks, conservative maximum-HP
settlement, and scheduled bulwark expiry, including failed-check costs and the
explicit deferred backlog. The Schedule kernel has logarithmic keyed operations;
this is not a claim that every world-tick subsystem has migrated to events.
Detailed contracts: [lighting](LIGHTING-CACHE-CONTRACT.md) and
[health/ownership/shared-state dependencies](dirty-contract.md). Measurement and
final integration results remain pending unless explicitly recorded in the
first-pass verification section. The universal function annotation audit remains
paused and unfinished.

## Creature identity and floor-local indexes
See [creature-identity-index.md](creature-identity-index.md) for variables, lookup/reconciliation/COW costs, retained slot high-water memory, admission rules and mutation boundaries.

### Possible-exit hover sequencing (October 30, 2026)

`non_euclidean_ui::possible_exit_marks` now selects the same dream observation run and current floor, bounds-checks the hover, checks the revision, and retrieves the hover tile's possible pocket/speed-up exits **before** requesting `vision::observe_terrain`. An empty candidate list returns immediately. A nonempty list still requires a full fresh observation, then the original `current || remembered` gate for both the hovered tile and each endpoint. Serialized topology alone never authorizes a visible mark. The retrieved candidate list is reused, avoiding a second group search on the relevant path. Portal observation, optical routing, knowledge, dream projection, aliases, travel and simulation rules are unchanged.

Per invocation, let D be the existing dream observation-run preparation cost (ordinary runs borrow in O(1); active projections clone run metadata/history and may prepare a missing dream layer), F the number of floors searched by `Run::floor`, P the number of non-Euclidean/speed-up pockets examined, C maximum cells per pocket, G groups in the matching pocket, S maximum sources per group, and E returned candidate exits. The pre-observation work is O(D + F + P log C + G S + E), with O(E) additional scratch beyond dream projection. Absent links or disabled revision require O(D + F), not a full tile/actor visibility scan. Thus the local applicability query is O(1) when links are absent; the complete wrapper is not universally O(1), because floor lookup/dream preparation still run. An unrelated hovered tile pays only topology lookup, without observation. Generated limits currently bound ordinary pockets and speed-up areas at eight each, cells at 48 per pocket and sources at two per group; the formula also states the cost without treating these authored limits as an asymptotic assumption.

Relevant exits still pay a full `vision::observe_terrain` call plus O(E) endpoint filtering/output. With the October31 optical/light fields, its current expected bound is O(C + V*(P*(F+B)+T+H+M) + A*B+L + S*(Q+V*R)), with O(V+H+P*V+S*V) fields/output. C includes exact dependency guards and cold field construction; V cells, P viewpoints, F floor lookup, B ability entries, T telepathy, H/M metadata, A actors, L placed lights, S disclosed sources, Q scalar emitter visibility and R ray length. Viewer-hash worst-case adds O(V*P²). See OPTICAL-VISIBILITY-CACHE.md for cold/warm C. Whole observations are not cached; geometry and lighting are. These are symbolic costs, not measured timings. Disabled runtime timing remains a no-op; enabled span bookkeeping adds its existing O(log labels) diagnostic overhead.

Focused regressions retain the pre-change implementation as a test-only oracle and compare missing links, unrelated tiles, linked sources/destinations, speed-up areas, portal combinations, hidden/remembered endpoints, legacy revision, out-of-bounds positions and differing waking/dream topology. They assert run/RNG immutability. A runtime-timing regression verifies zero `vision.observe_terrain` calls for absent/irrelevant/out-of-bounds cases versus one for relevant exits, and verifies that the old absent-link path did call observation. Existing alias/plane/renderer tests remain applicable. These tests do not imply a whole UI dirty-flag rewrite or universal complexity-annotation completion.

### Exact optical and actor visibility batching (October 31, 2026)
See [OPTICAL-VISIBILITY-CACHE.md](OPTICAL-VISIBILITY-CACHE.md) for per-function
cold/warm geometry costs, bounded observer caches, exact dependency guards,
scalar-query limitations and directed-ray semantics; see
[ACTOR-LIGHT-CACHE.md](ACTOR-LIGHT-CACHE.md) for excluded-source light fields.

### Exact-output bulk painting
See [painting-bulk-audit.md](painting-bulk-audit.md) for every modified painter
function, transitive pixel/raster work, cache dependencies and bounds, complete
painting dispatch audit, preserved RGBA/clip/rounding semantics and verification.

### Save and rewind export reuse (October 2026)
See [save-rewind-export-complexity.md](save-rewind-export-complexity.md) and [save-envelope-complexity.md](save-envelope-complexity.md) for bounded immutable-edge reuse, invalidation proof, direct envelope encoding and retained serialization/durability costs.

### Snapshot opening identity reuse
See [snapshot-opening-cache-reuse.md](snapshot-opening-cache-reuse.md) for the exact metadata/root guard, conservative external-write fallback, retained-edge eviction and added O(R + M) guard cost.

### Incremental durable command saves (October2026)
See [incremental-save-journal.md](incremental-save-journal.md). Routine changed saves encode/write only new command receipts but retain O(existing file bytes) validation reads; periodic and unknown-mutation checkpoints retain full export costs. Checkpoint/read/replay bounds, token ownership, incomplete-tail recovery, compatibility and durability limits are explicit. Do not label total save work O(delta) or omit periodic spikes.

### Seed replay policy compatibility
See [replay-policy-v2.md](replay-policy-v2.md): O(1) persisted boundary and per-command policy selection; no new world scan or save cadence.

### Cached visible-creature rail and UI hover (append trial)
The map keeps its existing tile size, aligned to the playfield's right edge; the
spare left margin below the Chronicle contains cached visible creatures. The list never covers the map or moves the player resource/ability controls.
Wide-floor minimum-left-width accommodation is pending designer confirmation.
`render::enemy_list::Layout` resolves/scrolls/hit-tests in O(1), O(1) space.
Painting visits at most viewport capacity R: O(R × measured text work + painted
pixels), with bounded temporary formatted strings. It consumes `View.creatures`
(the existing visibility-filtered observations); it performs no visibility,
pathfinding, actor lookup, or whole-world traversal. Hover stores observed ID bits;
right-click opens the existing inspector using that actor ID. Ability hover and
right-click share one cached-row hit test. Existing map Rally/Ally right-click
behavior is unchanged. DPI/resize input uses the existing logical-coordinate
transform, identical to rendering. Scroll offsets clamp when the observation
count shrinks. The existing inspector's observation/projection costs are unchanged.

Inventory command completion uses the actual `DispatchOutcome.took_turn`, not a
command-name guess: successful turn use closes the pack, free/failed use keeps it
open. A target selector temporarily hides the pack without a turn; cancellation
or rejection restores it, successful target use applies the same outcome rule.
Terminal status takes precedence over reopening. The completion decision is O(1)
plus error-message formatting; command execution and pack refresh costs unchanged.

### Revision40 shore currents (new runs only)
Legacy29–39 current generation, destinations, saved fields, and replay semantics
are unchanged. Desktop new genuine/debug runs explicitly select40; old startup
convenience APIs still default39. No old-run revision/floor migration occurs.

`shore_currents::repair` precomputes water/bank eligibility, builds a cardinal
multi-source BFS to safe banks, then classifies existing arrow routes as a
functional graph. Routes already ending at a safe bank are preserved. Trapping
cycles/tails/still pools use the BFS route; enclosed components receive zero
currents, not carved terrain. Generation is O(V×B+V) time/O(V) scratch, where B is
the actual terrain eligibility predicate cost (existing linear grate/permanent
fire scans, door key lookup, bounded portals and special solid fixtures). Three
bounded generation calls account for final authored setup overlays and later
portal placement; not a per-turn scan. No generation/simulation RNG draws added.

Runtime revision routing adds O(1) dispatch. Each current destination costs O(B).
An occupied shore landing can select another of at most four adjacent safe banks,
O(4B) plus existing occupancy HashSet lookup; no full-floor runtime search. Actor
sorting, actor lookup, normal movement/terrain arrival, item drift, and offscreen
simulation costs remain their existing costs and are not hidden in this bound.
Permanently blocked exits or no reachable bank are not an unconditional escape
guarantee. Old records keep original header/revision;40 uses its own admitted
recording build+revision pair and seed replay records40 explicitly.

### Display-only Chronicle articles and native Heartlessness coverage
R52 corpse badges distinguish harvest from dropping a heart. At death and
stomp a fixed-size provenance receipt is updated; on actual heart collection a
batched lookup scans A stored actors once and updates only matching actors.
Corpse-priority projection adds O(I) time and scratch for I visible item
cells; the ring reuses the existing clipped native-pixel geometry.
Doorway-wait narration is filtered before recent Chronicle grouping and archive
display. The recent window examines at most the bounded 80-entry run log to
retain up to 30 visible messages; save, replay, and event data are unchanged.
`chronicle::present` adds articles only to recognized simple combat phrases and
catalog species names. Canonical messages and archive/replay bytes are unchanged;
compaction groups original strings before decorating detached display text.
For C catalog names, fixed V recognized verb patterns, and L message bytes, the
conservative per-message bound is O(C V L), with O(L) output. The cached catalog
has one-time O(C log C) sorting plus name bytes; opening history applies the
transform to E entries, O(E C V L). This is not a universal grammar parser.

`Painter::heartless_marker` retains the existing glyph-specific authored center,
radii, colors, and observed-state eligibility. It now evaluates fixed 4×4 area
coverage per drawable pixel inside the clipped marker rectangle, instead of
expanding logical pixels into blocks. Time is O(P_marker), including the fixed
16 coverage samples per native pixel; extra working memory is O(1). The native
area scales with drawable DPI squared. No actor lookup, visibility query, asset
load, random draw, or gameplay mutation is introduced. The complete map painter
still pays its existing glyph/background/full-frame costs.

### Direct menu descent and optional seed (append trial UI)
The earlier six-row menu paragraph is historical. The current first row contains
Descend, difficulty and an optional seed field, all sharing painted/hit-tested
bounds. Hit testing, focus, fixed-count keyboard navigation and busy-state checks
are O(1). Seed edits/validation/copies and horizontal visible-tail measurement
are O(L) in seed bytes/glyphs (input capped at180 bytes); no generation occurs on
hover or editing. Blank/whitespace input calls the existing OS-random seed routine
once per accepted start. Entered input retains existing trim/validation semantics.
Descend enqueues the existing asynchronous generator directly, without another
seed modal. Enqueue cost is O(L+mode bytes), NOT floor-generation cost: generation,
engine validation and allocation retain their transitive documented costs. Loader
busy checks suppress repeated starts; cancellation/completion retain existing
ownership rules. Menu paint remains O(pixels rasterized + glyph work), including
the existing layered heart animation; it is not constant total work.

### Chronicle shutter and blessing hover (append trial UI)
Opening Chronicle acquires the existing history lines once, with history/archive
construction and formatting costs proportional to that query's inputs. The
shutter retains those lines, not another world snapshot. Animation state,
reversal, clipping bounds, header hit tests and scroll offset updates are O(1).
Each repaint draws only viewport-capacity lines: O(visible glyph work + clipped
panel pixels). Retained line memory is O(history text). The220ms cosmetic height
animation neither observes the world nor advances turns; reduced motion uses an
instant endpoint. Input is captured through opening/open/closing, so underlying
movement and clicks cannot leak through. Scroll position is retained/clamped.
Full-frame game/background paint can still occur beneath this panel; clipping
alone is not a whole-frame renderer cache. Blessing hover uses the same fixed
card bounds as clicking, O(1), changes only selection/presentation dirty state,
and does not choose an offer, dispatch a command, or advance time.

### Inventory hover invalidation (append trial UI)
`PackView::hover` is O(1) pointer state; identical coordinates do not request a
new hover redraw. `configure` checks a fixed layout/style signature and metric
presence before returning on a warm cache, O(1) fixed-field work (bounded style
strings), rather than revisiting all groups on every queued event. A content,
font/style or layout miss performs label metric work and selected-detail wrapping:
O(group label bytes + detail wrapping work), with their metric/string storage.
`refresh` remains O(I²+I×A+G log G+T) time / O(I+T) extra space, where I is inventory
items, A relevant abilities, G grouped rows and T formatted text. It runs on
content/action invalidation, not inventory mouse motion. Hover-only draws skip
world observation and pack content refresh; moving tooltips still require paint.
Actual full software background/game/pack paint remains O(pixels + glyph work)
per repaint; no background pixel cache or GPU optimization is claimed here.
Map-hover route calculation is unchanged, not covered by the inventory shortcut.
Counters/spans `desktop.ui.observe_world`, `desktop.pack.refresh`, and
`desktop.pack.configure_rebuild` separate these costs in diagnostic builds.

### Cached idle fire and restrained Flesh contrast (append trial UI)
Observation creates a fixed four-anchor fire descriptor for a current-known fire
cell with nonzero animation speed. Existing material/style/lighting/tone transforms
are applied at descriptor creation; cost is four bounded evaluations per fire,
including existing settings/object lookups. Descriptor space is O(F) for F
observed animated fires. Idle paint interpolates those cached anchors from the
cosmetic clock/phase in O(1) per painted fire cell. It does not access Run, consume
gameplay RNG, observe visibility, or rebuild the detached terrain/light field.
Default speed1 is eligible; speed0 has no descriptor; remembered-only fires are
not animated. The existing reduced-motion zero-clock path is static. Explicit
custom fire color remains authoritative, subject to existing lighting/tone.
This is NOT O(F) total-frame rendering: the existing approximately83ms idle
cadence may repaint the entire software framebuffer, with its O(pixels+glyphs)
cost. Other explicitly animated material/light settings retain their existing
terrain-frame/cache-miss costs. The default wall/floor Flesh palette changes
only fixed unedited color channels, O(1) per material; crimson water, special
substrates, authored overrides and other biome palettes are preserved. No
simulation generation, palette RNG, save or gameplay behavior changes occur.

### Append-recording trial versus historical save complexity
Earlier incremental-save/checkpoint paragraphs describe their historical builds,
not this separate append-recording trial. The trial uses a lifetime OS ownership
lock and bounded append guards; ordinary flush does not reread the growing full
save and no longer forces the old32-record/1MiB periodic rollover. Encoding and
durable append still cost the actual new record bytes plus OS write/sync latency;
initial checkpoints and explicit recovery/capacity paths retain whole-envelope
costs. Cold recovery still parses/replays its bounded retained recording; it is
not O(1). File/record safety caps, atomic replacement/recovery, external-edit
handling and measured platform caveats are covered by the trial's storage report.
These UI changes do not weaken durability or change save cadence. No Windows
runtime performance or visual result is implied by Linux headless tests.

#### Idle-family controls and dream eligibility follow-up
Current-known water/shallow-water overlays now honor the corresponding
`terrain.water.animationSpeed` / `terrain.shallow-water.animationSpeed` zero.
Gas overlays honor `terrain.gas:<kind>.animationSpeed`; unregistered gas families
retain the historical default1. Only zero/nonzero gating changed: existing
nonzero overlay waveforms/rates remain unchanged. Wall torch overlays use the
same resolved object style as the source: `entities.objects.<source>.animationSpeed`,
with existing `objects.light`/`entities.defaults` inheritance. Thus default resolved
zero suppresses the old unconditional torch flicker; an inherited nonzero speed
can still enable it under existing style semantics. These bounded profile/style
lookups occur when observing cells, not per idle pixel/frame. Global reduced
motion still supplies clock0 and suppresses ambient redraw scheduling.

`cached_view_needs_idle_paint` now admits a cached dreamwalking View directly,
O(1) for that branch, so existing monochrome dream flicker no longer relies on
an incidental water/fire/effect redraw. Otherwise existing eligibility costs
O(registered profile entries + cached cells) worst case; this is not a world
scan. Dream painting itself retains existing O(cached cells) temporary copies
and O(pixels+glyphs) rasterization, no new visibility/Run/RNG work. Portals remain
static; no generic magic/lava family or static-wall animation was introduced.
Custom nondefault material animation can still invoke existing detached terrain
and lighting reconstruction; this follow-up does not optimize that older path.

## Revision41: retiring Revert retention (implementation in progress)

New-rule temporal dispatch bypasses all five-turn History opening/closing capture,
mutation-journal scope, pending-haste Snapshot creation, rebasing and history export.
The revision check is O(1). Ordinary checkpoint encoding serializes current Run
(and an active explicit Future anchor, if any), not five previous worlds. History
import rejects nonempty retired history for revision41 rather than discarding it.
Legacy29–40 snapshot, inverse and compatibility code remains for exact old saves
and replays, but is not traversed by new-rule ordinary commands.

This does not remove command transaction candidate clones or phase-before rollback
clones in engine.rs: they protect errors/free-phase rollback independently of Revert.
Their existing metadata/COW copy and eventual destruction costs remain. Future
activation/restoration still performs explicit whole-state serialization/deserialization
O(S) on demand; no claim that all actions or checkpoints are O(1). Test-only
thread-local Snapshot counters assert zero calls for new-rule ordinary commands,
save/reload and Future, while legacy tests retain their original capture behavior.

### Timeweave state primitive (revision41 integration in progress)
See [timeweave-state.md](timeweave-state.md). Private chronological VecDeque plus
cell HashMap gives expected O(1) lookup / amortized O(1+k) append with collision
prefix removal, where k is removed segments. No survivor shifting or endpoint
diff. Serialization and index reconstruction are O(L) for L live segments;
retained capacity is O(H), H peak allocated entries, and hash worst cases are
not constant. Optional Arc-backed Run state clones O(1), but shared first-write
COW detachment costs O(H+L) including retained hash capacity, potentially on each command candidate. Therefore
this is not an O(1) end-to-end movement claim. The primitive does not decide
arrow ownership, free-step acceptance, forced movement or crossing rules.

Timeweave presentation is separately specified in [timeweave-visual.md](timeweave-visual.md):
indexed lookup during existing observed-cell traversal, cached direction painted without
Run/trail iteration, preserving current visibility and floor/plane identity.

Timeweave ordinary walking performs expected O(1) reverse lookup (at most eight
under confusion). Only a possible reverse edge clones the Run candidate and event
batch: O(R + E + B) owned Run bytes R, events E and explosion-dedup entries B, plus
movement-kernel transitive work and touched COW detachments. A rejected probe can
execute that movement work again on the unchanged charged path; this is not O(1)
whole-command cost. RAII restores exact ordered event/dedup state on failure/unwind.
Only command-scoped actual displacement records; public forced/current movement
does not. Source collision removes O(k) old prefix entries expected; first shared
trail mutation still costs O(H+L). Status/dream tests and tail timings must establish
actual overhead before release. Nonwalking policy remains explicitly pending.

New41 Future restore parses and validates the explicit anchor once before changing
live command bookkeeping or replay ownership. Cost is O(anchor bytes + Run/domain
admission + F + L); then it transfers replay and identity and applies existing cost
policy. This is an explicit Future action, not ordinary per-turn retained history.

Revision41 checkpoint/canonical-export boundaries additionally validate trail cell
geometry in O(F+L) expected time and O(F) scratch (physical+dream floors F, live
segments L), so direct public/admin edits cannot emit an unreadable trail checkpoint.
This is not called per normal append/movement. Legacy absent-trail saves do not pay
this traversal. Current-state JSON serialization still scales with world size S.

Desktop checkpoint envelopes now use `Engine::checkpoint_state`: revision41 borrows
current Run after boundary validation; legacy revisions own the exported-history
projection. The new-rule save no longer clones Run merely to satisfy the retired
history-export API. The existing canonical_run API still returns ownership when
explicitly requested by other callers/oracles. Serialization bytes are unchanged.

Timeweave voluntary status/dream follow-up supersedes the initial conservative
preflight above. Relevant reverse eligibility is O(F+abilities+8 expected cell
lookups), without RNG; only relevant cases clone the candidate Run and full
pending event Batch. That clone costs O(R+E+B), R deep-owned metadata/archive/
replay/player/inventory, E ordinary events<=128 and B ordered explosion entries,
plus normal movement/COW work. Failed probes repeat movement on the charged path
but restore all original Run/RNG/events. Accepted reverse movement calls normal
arrival/finish without time prelude or trail append. Normal charged moves record
actual resolved edges; statuses/confusion/dream no longer rely on requested dx/dy.
Forced/portal/stair policy remains pending. See timeweave-state.md for boundaries
and event-buffer ordering/cap safeguards; no O(1) total-command claim is made.

## Revision41 native Future performance follow-up

See [Native Future anchor](future-native-anchor.md) for authority, compatibility and proof details. Finite capture performs O(F+M+N) COW-handle/deep-metadata cloning plus a no-output canonical numeric traversal; no JSON is built to establish the anchor. Fresh native restore still pays O(F+M+inventory-order+validation), including the existing full JSON Value validation, not O(1). Certified imported anchors already validated on load skip only duplicate storage validation; clone/order normalization and temporal revision/history/trail guards remain. No validation cost is shifted to fresh capture. First wire encoding and every outer save copy/escape remain O(S); a derived inner text cache makes only subsequent inner lookup O(1). Imported41 anchors warm during load, with failed/exceptional and<=40 text behavior retained. Holding the second native state retains old shared blocks and can cause O(L) trail detachment on later mutation. Peak/retained memory includes deep archive/metadata, diverged blocks and cached encoded text; allocation-only example measures requested heap bytes separately from uninstrumented latency. PartialEq is conservative identity/equal-text equivalence and never forces serialization. Candidate gate/timing results must be recorded separately; this entry asserts no measured speedup or RAM estimate.

## Retained software painting (revision41 performance trial)

See [retained-rendering.md](retained-rendering.md). New-view descriptor discovery is O(C log C + descriptor bytes + spatial bucket entries); old/new region merging is bounded to eight rectangles (ordinary O(R²), capacity-pair/overlap-closure search O(N⁴), N<=9; contact-only L/corner shapes stay separate). Raster work and row copies scale with selected primitive footprints and damaged area Q, with conservative overdraw. Retained clean base adds exactly4P bytes plus descriptors/indexes. UI metadata and effects retain their documented scan/layout costs; this is not O(changed pixels) for all upstream processing. Unknown complex modes and global layout/profile changes have explicit full-frame fallback.

## Immutable compiled appearance projection

See [appearance-compiled.md](appearance-compiled.md): cold edit/load-time typed scalar and terrain inheritance compilation, owned-memory accounting, expected hash lookup costs, and per-observation original-ground neighbor reuse. JSON authority/default equality and dynamic light/noise semantics are preserved; no per-frame profile hash or cache rebuild.

## Full painting restored (revision41, user-directed rollback)

See [full-painting-runtime.md](full-painting-runtime.md). Production main no longer
allocates a retained4P-byte clean base, captures/indexes Scene descriptors, plans
regions or subuploads textures. Each requested redraw clears O(P), rasterizes the
full composition with existing cell/text/glyph/layer costs, and uploads4P bytes.
Cached View/cosmetic invalidation, compiled appearance and native Future remain.
Idle frames still skip drawing unless eligible animation requests it. Experimental
retained helpers/benchmarks remain non-active evidence. This is an explicit user
policy choice following mixed measured results, not a claim all full frames win.

## Opt-in tile-paint prototype V2 (source analysis; verification pending)

See [tile-paint-prototype.md](tile-paint-prototype.md) for the separate
`CORPSE_TILE_PAINT=1` candidate; unset keeps the full-paint policy above. New-View
borrowed comparison and footprint preparation cost O(N + C + B + L): N map slots,
C paint layers, B compared/footprint text bytes and L other variable list entries.
Two reusable flat indices replace neither the cached View nor upstream observation.
There are no planner Scene/Cell clones or extra 4F-byte clean-base framebuffer,
but existing global animated/Dreamwalking paint branches can still clone Cells.

V2 replaces only V1's interval-push/sort builder with reusable row-major u64 mask
words. For W native columns, H rows and Q = ceil(W / 64), reset costs O(H + H×Q),
rectangle union O(K + marks + Σ h_r×q_r) for each clipped rectangle's height and
covered words, and exact row-run emission O(H×Q + R). R includes within-word runs
before joining across words. Duplicate rectangles still repeat word operations,
but allocate no duplicate intervals and require no sorting. `raw_span_entries`
now counts logical rectangle-row contributions, not allocated entries; separate
bitset touched/scanned/capacity counters describe the latest mask construction.
Panel dependency closure repeats construction and per-row intersection tests for
bounded D dependencies. Every nonempty candidate
base paint makes one full C order pass with footprint intersection per layer,
then existing layout/glyph helpers and clipped raster work O(P), P including
overdraw. A row-mask intersection is not O(1); no O(K)-only frame claim is made.
Effects still have separate generation/projection work, including the existing
Painter BTreeMap, even though tile indexing uses flat vectors.

Retained index/mark/span capacity is O(N + C + K + marks + H + retained spans),
plus 8×H×ceil(W/64) live mask bytes (capacity can exceed this), overlay text/effects
and existing renderer caches. Mask words are not another RGBA framebuffer copy.
Normal upload planning
costs O(H + merged spans), allocates U rectangles and requests 4A bytes for union
area A; over 128 requested rectangles switches to one full 4F-byte upload without
expanding CPU paint damage. SDL/driver upload/present costs and allocations are
not constant wall time. Full context/modal fallbacks remain. V1 runtime regression
evidence remains in `reports/tile-paint-prototype/timing-v1` and `attribution-v1`
under the trial root, linked from the subsystem document; no V2 speedup is inferred.
V2 exact pixel tests, upload readback, timing and integration gates are pending;
this entry establishes source bounds, not performance or release certification.

## Scoped metadata save records
See [scoped-save-records.md](scoped-save-records.md): O(I) selected order capture,
O(D) run-ID capture, bounded ordered operation batches, exact checkpoint metadata
restoration, and one-use O(B) resume verification. No whole-world change discovery.

### Revision42 forgiving-start numerical birth overlays

- Natural new actor: O(R) time for R frozen profile rows, followed by two numeric
  assignments and at most3 existing gas-condition writes. First lookup lazily
  parses O(B) bytes once and retains O(B) profile storage. No world/floor scan,
  per-turn scaling, RNG call, saved-actor retuning, or player reward change.
- Gas-condition insertion hashes fixed short keys; expected amortized O(1), but
  an IndexMap reallocation can copy O(M) existing condition entries. Birth-only,
  at most3 keys per actor. Explicit child context may construct O(kind bytes)
  key and do one additional O(R) lookup. Queen applies natural then final child
  profile; child values replace rather than multiply the natural result.
- Early Zillennia pulse amount: O(1) quality/revision selection inside existing
  pulse path. Existing radius/LOS/victim traversal, damage, cooldown and energy
  complexity is unchanged. Revision<=41 retains original raw amounts.
- Offline calibration is not a runtime dependency. Its expense is encounters ×
  capped commands × native dispatch cost (AI, statuses, summons, environment).
  The command cap does not bound actor growth to a constant or establish a
  completed encounter; deaths/censoring are reported separately. Birth numeric
  fit comparisons are not measurements of interactive frame performance.

Revision 43 admission and persistence: see [revision43-plumbing.md](revision43-plumbing.md) for fixed whitelist/header costs and unchanged resume/birth-profile complexity.

Revision43 item/action wording, Honey, deliberate chasm entry, Fury calibration and numeric inventory previews: see [revision43-item-actions.md](revision43-item-actions.md). Player-owned hallucination exposure: see [revision43-hallucination.md](revision43-hallucination.md).

Revision 44 admission and persistence: see [revision44-plumbing.md](revision44-plumbing.md) for unchanged bounded-header/resume costs and fixed profile inheritance.

Revision 44 display-only wording: see [revision44-wording.md](revision44-wording.md) for bounded presentation overrides without catalog or simulation mutation.

Revision44 bleeding: see [revision44-bleeding.md](revision44-bleeding.md).

- [Revision45 display wording](revision45-wording.md): fixed display-only text overrides.

Revision 45 admission and persistence: see [revision45-plumbing.md](revision45-plumbing.md) for fixed revision guards, exact header IDs, inherited profiles and unchanged bounded recording/resume costs.

### Revision 45 Nova funding
See [revision45-nova.md](revision45-nova.md#complexity) for the constant-cost
revision dispatch at every existing post-attack funding point, unchanged shared
ownership/explosion/resource-history transitive work, Unspeak's catalog scan,
read-only desktop projections, persistence boundaries and controlled coverage.

- [Revision46 Shine display text](revision46-wording.md): fixed description adapter.

- [Revision46 Shine emission suppression](SHINE-SUPPRESSION-REVISION46.md): revision-aware emitter filtering across gameplay, terrain and appearance.

Revision 46 admission and persistence: see [revision46-plumbing.md](revision46-plumbing.md) for fixed revision guards, exact header IDs, inherited profiles and unchanged bounded recording/resume costs.

- [Revision 46 wording refresh](revision46-wording-refresh.md): five display-only descriptions and retained live adaptation suffix.

- Hover observation reuse: see [hover-observation-reuse.md](hover-observation-reuse.md). Existing view terrain is reused only under current-world admission; route semantics unchanged.

- Test-only native paint attribution: see [native-paint-profiling.md](native-paint-profiling.md) for bounded exclusive counters, collector OFF control, actual-frame workloads and measurement limits.

- Native unit-opacity mask painting: the common alpha==1 path uses exact byte coverage instead of per-pixel floating multiply/round. O(clipped mask pixels) time and O(1) scratch remain, including fragmented clip spans; arbitrary alpha paths retain prior casts. See `native-paint-profiling.md` for separate instrumented attribution and OFF timing controls. No extra retained framebuffer or glyph-color cache.


- Candidate sparse ASCII unit-opacity painting: only ASCII map masks with alpha==1
  skip transparent coverage; destination alpha is preserved. Each cache insertion
  adds one O(W*H) scan over exact native byte coverage and retains H+1 u32 row
  offsets plus R pairs of i32 run endpoints, R <= H*ceil(W/2). Added payload is
  4*(H+1)+8*R bytes per mask, with vector capacity/allocator overhead separate.
  Existing at-most-513-entry map-mask cache threshold and resize invalidation are
  unchanged. Warm drawing costs O(Hc + sum(log(1+Rrow)) over clipped tile segments
  + Rc + Pc), where Hc is visited clipped rows, Rc intersected nonzero runs and Pc
  visited nonzero samples; native tile row-segment lookup/traversal is additional
  existing work. No per-frame full-mask scan or framebuffer/tint cache is added.
  Non-unit ASCII and forced-alpha text/compound paths keep previous traversal.
  See [sparse-ascii-candidate.md](sparse-ascii-candidate.md) for exactness tests,
  same-binary full/sparse OFF/ON controls, occupancy diagnostics and unverified
  performance status. This is not an O(changed pixels)-only whole-paint claim.


- Candidate fused opaque game background: replace consecutive clear(base) and
  rect(fog) passes with one exact clear-selected native pass. Q clear-selected
  pixels and A fog-covered pixels require Q rather than Q+A writes. Cost is
  O(H+S+Q) plus existing row-span lookup work, versus O(H+S+Q+A); H is bounded
  damage rows and S selected tile spans. Each span splits into at most three
  contiguous color slices. Scratch O(1), no cache/allocation/extra framebuffer.
  Logical clipping still affects only fog, native damage/tile clipping affects
  both, and written alpha remains 255. Both full-frame algorithms remain O(P)
  for P native framebuffer pixels. See [fused-background-candidate.md](fused-background-candidate.md)
  and the independently controlled eight-arm profile in sparse-ascii-candidate.md.
  Source-reviewed only; exactness tests and timing remain pending.

## Accelerated desktop drawing

See [gpu-rendering.md](gpu-rendering.md) for Q ordered-quad construction,
mask-cache miss costs, bounded atlas allocation, GPU overdraw, resize behavior
and software fallback. Simulation, observation, persistence and replay
complexities are unchanged. Hardware timing must be measured independently.

## Static Bestiary scheduling

Screen scheduling checks are O(1). Static Bestiary pages no longer redraw solely
on a cosmetic timer; input, page changes and window exposure still request the
existing full paint. Artwork and simulation behavior are unchanged.



- Candidate CPU Chronicle fade LUT cache: one exact (native fade height, RGB)
  entry retains 768 bytes per admitted row, maximum 2048 rows/1,572,864 requested
  payload bytes, plus Vec/allocator overhead. Let H be full fade height, R rows
  selected by damage, S tile spans and P their pixel visits. Old work is
  O(256*R+S+P), warm cached work O(R+S+P), both plus existing row-span lookup.
  A cold key adds O(256*H) table construction and O(H) allocation/initialization;
  clips can select R<<H, so cold heavily clipped work can increase. Capacity is
  reused, resize invalidates the key, hits allocate nothing, and unsupported
  heights or failed reserve use the exact old row-table path. Top/width/clips are
  not cache dependencies. Original floating fade/composite rounding and alpha
  preservation remain. No GPU primitive, framebuffer cache, simulation or paint
  policy changes. See the staged Chronicle-cache subsystem note and benchmark
  for exact tests, cold/warm/first-allocation controls and unverified timings.

## Opaque Pack map coverage

Pack coverage adds O(C) bounded map-cell checks and O(1) scratch. Fully covered
plain ASCII cells with default typography use a conservative three-cell footprint
and can skip hidden drawing; custom/annotated/compound/boundary cases retain full
painting. The already-prepared Pack state is reused in the same frame. No scene
cache, world mutation or tile-compositor change is introduced.

### Chronicle attack vocabulary (November 3, 2026)
Revision46+ attack wording is a pure action/identity projection, described in
[attack-wording-r46.md](attack-wording-r46.md). Per attempted observed attack,
known-identity lookup is O(C × L) for C fixed catalog entries and L identity bytes,
live projectile ability checks add O(B × L) for B actor abilities, and the implicit
player main-hand path adds an O(I) inventory scan. Explicit hand strikes reuse the
already-resolved item. Formatting is O(M) time/output bytes for message length M,
with O(1) other scratch. Catalog initialization retains its existing O(C × L)
allocation once. No RNG, new persisted state or per-frame work is introduced.
Desktop article projection retains O(C × V × M) worst-case time and O(M) output,
with the fixed vocabulary V expanded and three new-creature names included in
its existing once-loaded sorted catalog. No measured timing claim is made.

## Revision 46 Chronicle noise reductions

The per-command, per-creature-turn and per-death message gates add O(1) time and
O(1) memory, with no new scans, caches, serialized fields or rendering work.
Skipped messages avoid the existing text/allocation/archive costs; retained
messages keep their existing visibility/name lookup and append costs. See
[Chronicle noise reductions](chronicle-noise-revision46.md) for preserved effects,
revision scope and deterministic regression coverage. No timing claim is made.

## UI sequence item 2: persistent full visible-creature previews

The playfield reserves 252 logical pixels to the left and remains right-aligned
beside the existing right-edge HUD. Layout is O(1). Rounded tile heights are
checked before positioning so the final row stays inside the playfield.
Only `View.creatures`, already filtered by perception, supplies previews. There
are no new world, visibility, simulation, save, replay or RNG queries.

Let C be admitted creatures, T their displayed characters, S their statuses and
M their measured conditions. Preparing measured cards costs O(T² + S*M) worst
case (existing word/long-token wrapper repeatedly measures prefixes), O(T+C+S)
extra storage. This occurs on a painted preview panel or a pointer event inside
its rows; unrelated map pointer movement does not prepare cards. Hit testing is
O(C); pixel scrolling/clamping is O(1) after preparation. Painting is O(C+L+P),
where L is rendered glyph-mask/metric work in intersecting cards and P clipped
native pixels; GPU recording substitutes ordered rectangle/text-mask commands.
The whole list is pixel-scrollable, so neither dense populations nor one very
tall condition list loses content. Resource/status fields participate in both
retained and tile damage comparisons. No timing claim has been made.

Current environment lacks Cargo/Rust. Native test invocation is recorded as
blocked, not passed. New deterministic coverage lives in enemy_list.rs; the
arithmetic mirror is not a substitute for native or Windows verification.

## UI sequence item 3: corpse X overlays

`Cell.corpse` is copied from observed actor death state; the living heartless
ring and sleeping body semantics remain separate. The X preserves the creature
glyph and uses the existing native-pixel path for GPU and software, including
combat-effect repaint. A constant 4×4 coverage grid per clipped marker pixel
costs O(P) time and O(1) scratch, where P is the marker's drawable-pixel bounds.
GPU mode records up to one clipped quad per covered pixel, as the heartless
marker already does. Retained/tile comparisons include the death flag. No
simulation, visibility, replay or RNG behavior changes. Tests cover observation,
CPU-buffer independence, clipping and retained/tile parity; native execution is
currently blocked by missing Cargo, not reported as passed.

## UI sequence item 4: floor-blood dot color

Both compiled and reference terrain styling use `world_features::blood_dot`.
It retains the ground's punctuation and the blood foreground appearance, but
removes the feature background and the extra default flesh-palette stain. Dry
and bloody versions of the same observed floor cell therefore have identical
backgrounds. Bloody walls, blood-contaminated water and Ichor retain their own
separate behavior. Authored blood background/glyph values remain stored but no
longer paint a floor tile fill or replace its punctuation.

This is O(1) additional work/scratch per admitted blood cell after existing
material resolution, with no new scan/cache, random draw or live-state query.
Regression coverage compares dry/wet background bytes and compiled/reference
output under direct/remembered knowledge, default/custom appearance and fixed
cosmetic times. Native execution is blocked until the toolchain is available.

## UI sequence item 8: compact dialogs

Authored modal sheets render at 88% of their former linear scale (77.44% area)
using one centered native transform. This reduces creature inspection, ability
detail, hazard confirmation, blessing and debug dialogs without discarding text,
changing scroll ranges or changing their authored controls. Input inversion and
retained damage use that identical transform. The main menu, gameplay canvas,
map tiles and nonmodal hover tooltip are not rescaled. Pack uses a 760×500 maximum
instead of 900×760, and its existing measured layout and separate list/detail
scrolling remain the input/paint authority.

Transform, inverse input and damage mapping are O(1), with no extra framebuffer.
Text, mask-cache and SDL work retain their existing complexity; rendered pixel
area is smaller, but no speed-up is claimed. The previous scale/origin/canvas are
restored after each modal, including repeated nested transforms. Test coverage
includes fractional/native/DPI inverse mapping, selection rectangles, repeated
open/render/dismiss repaint and saved-state identity. Toolchain absence still
blocks native execution and screenshot generation at this checkpoint.

## Revision 50 desktop metadata and targeting

See [Revision 50 UI](revision50-ui.md) for metadata, category projection, multiple-grant text and two-stage targeting cost bounds and pending verification.

## Revision 50 Grappler

See [Grappler movement complexity](revision50-grappler-complexity.md) for actor-group planning, forced-movement hook costs, bounds, and unverified test status.

- Revision 50 amulet resource accounting and cumulative direct-hit Reaping: [coverage and complexity](revision50-amulet-accounting.md).

## Revision 50 Sparkle and All-Seeing Eyes

See [preserved blessing execution, action boundaries and complexity](revision50-preserved-edges.md). The change separates action completion from world-turn advancement, preserves explicit no-action commands and snapshots, and admits permanent Sparkle sources on every visible terrain type. Native tests are authored but execution remains blocked by the unavailable toolchain.

- Revision 50 ability reserves: [accounting, refill and integration](revision50-ability-energy.md).

- Revision 50 named rings: [fixed-power identity, regeneration and complexity](revision50-rings.md).

## Revision 50 fixed ring illumination

See [ring illumination](revision50-ring-lighting.md) for exact scalar/cached costs,
portal geometry reuse, actor-only Stealth, ring-source dependency tracking and the
explicit additive Light/Darkness Aura interaction. No quality scaling or rarity is inferred.

## Revision 50 combat and resistance rings

See [combat, movement and resistance rings](revision50-combat-rings.md) for
fixed-power packet/melee integration, run-aware flight, immediate equipment-loss
settlement, inventory-query costs and the unimplemented corrosion-status boundary.

## Revision 50 Ice Immunity

See [Ice Immunity scope and complexity](revision50-ice-immunity.md) for the
source-specific Yeti guard, pre-RNG footing guard, shared equipment ownership,
preserved non-cold statuses and snow behavior, and incremental inventory-query
costs. Existing portal adjacency, actor lookup and COW costs are included rather
than treating the chill pass as constant-time. Native execution is coordinated
separately; authored fixtures alone do not certify release admission.

- Revision 50 stat/status rings: [Willpower, Protection, Slaying, Sustain Strength and complexity](revision50-stat-status-rings.md).

- Revision 50 Mini-Me fractional stat preservation: [snapshot semantics, bounded check and regression scope](mini-me-coverage.md).

- Revision 50 amulet-only cooldown scope and fixed Bastidion hallucinogenic gas:
  [spawn/emission/offscreen audit, historical RNG compatibility, root recoloring,
  full-pass complexity and regression targets](revision50-cooldowns-bastidion.md).

### R50 targeted amulet effects (draft)

See [revision50-amulet-effects.md](revision50-amulet-effects.md#complexity) for admission, extended spatial-ray/portal planning, full fire-adapter closure, status/offscreen clocks, arrival hooks and knowledge-filtered UI projection costs. This entry does not open R50 release admission. Timing remains unmeasured.

- R50 Teleportation and Floods: [wave/teleport subsystem](revision50-wave-teleport-complexity.md), including destination enumeration, bounded wave geometry, forced-movement and visibility helper costs.

- R50 Beckoning, Discord, Obstruction and Roots: [control amulet boundaries and complexity](revision50-control-amulets.md#complexity), including visibility, actor selection, forced movement, temporary terrain COW and offscreen clocks.

- R50 item hover knowledge and concise copy: [projection, naming and observation boundaries](revision50-item-knowledge.md#complexity), including unchanged Pack grouping/visibility costs, full-tooltip font/raster work and legacy discovery preservation.

- R50 potions, darkness and permanent ice: [roster, exact scope, clocks, lighting,
  terrain/heat costs, persistence and verification](revision50-potions.md#complexity).

- R50 startup identity and persisted-grant boundaries: [scope and complexity](revision50-release-boundaries.md).

- R50 player-power command admission: [scope and complexity](revision50-player-power-admission.md).

- R50 recording protocol: [header identity, grammar, resume boundary and complexity](revision50-recording-protocol.md).

- R50 release admission: [version gates, preserved defaults, acceptance checks and complexity](revision50-release-admission.md).


### Revision 50 authored reserve acquisition and split payment
See [revision50-ability-energy.md](revision50-ability-energy.md#complexity-and-limits)
for explicit hash-collision bounds, dormant acquisition, real pickup/offer/
Remember/Tinker/level-up frequencies, owner resolution and Actor cloning costs,
transactional paid-Future Run/anchor/COW costs, and read-only UI projection.
[Amulet accounting](revision50-amulet-accounting.md#complexity) retains constant
scalar split planning and revalidation plus inventory/blessing/history/sync costs.
No measured timing or global R50 release admission is claimed by these changes.

Revision 50 production loot, unlimited special-item carrying and Masque naming:
see [revision50-items-generation.md](revision50-items-generation.md) for fixed
selection arrays, legacy RNG isolation, category admission, persistence decode,
UI capture and unbounded inventory costs. Source regressions await the parent’s
scheduled build; this entry does not claim playable or platform verification.


### R50 final terrain/caster choices

Shared-wall damage, no-route hostile breach fields, persisted terrain validation
and whole-object Shattering are detailed in
[revision50-terrain-pursuit-integration.md](revision50-terrain-pursuit-integration.md).
The approved independent-lane Floods policy and activation regressions are in
[revision50-wave-teleport-complexity.md](revision50-wave-teleport-complexity.md).
Birth-only explicit caster marking adds O(B) ability inspection, no gameplay RNG
or per-frame work; see [revision50-combat-coverage.md](revision50-combat-coverage.md).
These are analytical bounds, not measured performance or release evidence.

R50 deeper synthetic/debug inventory follow-up is recorded in
[revision50-items-generation.md](revision50-items-generation.md#deeper-syntheticdebug-loadout-follow-up):
revisioned two-starter selection, special-item carrying, finite stash admission,
atomic acquisition preflight and the inherited O(source entries × selected
items) provenance filter when R50 keeps all trinkets. New subsystem tests cover
floor6/floor40 fixtures; full R50 public-startup acceptance remains separate.

### R50 Windows distribution packaging
See [revision50-windows-packaging.md](revision50-windows-packaging.md#complexity-and-packaging-only-checks)
for source/evidence hashing, PE strip comparison, embedded-resource searches,
SDK archive verification, deterministic ZIP/read-back time and peak-memory
costs. These are offline packaging operations, not gameplay or renderer work.


### R50 five-section carried inventory correction (2026-11-06)
See [carried inventory](revision50-carried-inventory.md#complexity) for independent
category accounting, automatic carried-jewelry grants/stats/effects, pickup
settlement, non-destructive capacity-loss previews, debug/save boundaries and
Pack counting. It supersedes older shared-jewelry and unlimited-worn assumptions.
I remains unbounded because trinkets are unlimited, although each jewelry category
now has an authored finite cap. No measured timing is claimed by source checks.


R50 carried-jewelry loadout and persistence corrections: see
[deeper loadout costs](revision50-items-generation.md#deeper-syntheticdebug-loadout-follow-up)
for preparation-only candidate staging, one selected-loadout ownership settlement,
transactional debug mutations and per-category UI scans; see
[persistence boundaries](revision50-release-boundaries.md#complexity) for strict
quantity admission, revision-aware migration health scans and unchanged
Future/snapshot validation. Tests are authored for coordinated execution, not a
measured performance or platform claim.

Revision 50 creature naming and selected default glyphs: see [revision50-creature-identity.md](revision50-creature-identity.md) for stable-ID/heart compatibility, boundary-only alias migration, observation-only exact-black outlines, profile preservation, native raster/cache costs and regression scope.

## R50 creature identity ranks
The knowledge-filtered map adapter selects approved ranks for Ichor fish/sharks/leviathans (1–3), Zillennia (1–10), and Nightmares (1–8). Per observed actor, bounded kind/quality matching is O(1) time and space; rendering adds at most two rank digits. For A observed actors the adapter remains O(A) excluding pre-existing visibility work. Rank is presentation metadata and does not mutate saves or combat power. When an actor has an ally/clone order, rank occupies the superscript and the order occupies the existing subscript; corpses/bodies/older revisions suppress the new rank. Existing damage-rectangle spill handling includes both scripts. Verification: deterministic family/quality/marker tests and native render captures; visual legibility must be inspected, not inferred from matching.

### Quiet UI and native display follow-up (R51 build)
See [quiet-ui-r51.md](quiet-ui-r51.md) for observed-only two-way hover, right-aligned measured metadata, compact conditions, smaller corpse markers, inherited player backgrounds, three responsive toolbar targets and Auto/native pixels. It names input sizes, frequency, bounded limits, raster cost and unverified hardware behavior. No simulation or save-format changes.

### R50 activation-turn upkeep prepayment

See [ability reserves, activation-turn correction](revision50-ability-energy.md)
for keyed condition storage, per-activation/per-upkeep lookup bounds, and existing
reserve/Actor clone/journal transitive costs. No rendering or RNG work added.

### R51 persistent Bestiary discovery

Player-level `profile.json.bestiary.json` (or explicit profile basename plus
`.bestiary.json`) is independent of Run/save/replay serialization. At an actual
fresh gameplay observation, discovery intersects V admitted sidebar rows with A
current-floor actors, then validates against C immutable creature keys. Set work
is O(V log V + A log V + V*C), extra O(V) plus apparent-identity projections
(including their existing actor-copy and disguise lookup costs). No additional
visibility scan or gameplay RNG is performed; sensed, sleeping, nightmare and
hallucinated rows are excluded. New records/read-state changes serialize S seen
keys in O(S) bytes/time and atomic filesystem write+sync costs; each insertion is
O(log S). Input file reads are capped at 1 MiB; malformed files are preserved.
Recent Chronicle composition is O(L + B*L), where L is the bounded recent log
length and B is discovery batches in this application session (bounded by C).
Expanded history insertion is O(H + B*H), H archived messages. No canonical Run
or tracked command log is mutated. Unknown-page paint retains ordinary sheet
background/navigation and does not decode creature art or emit secret prose.
Bounds are analytical, not measured timings; run desktop bestiary_progress and
manual discovery_privacy tests, plus full workspace tests and render captures.

## Revision 51 stat display units
`stat_presentation::bonus` and `enemy_damage` are O(1) time/extra memory per
shown stat (two bounded actor-condition lookups for enemy damage). Pack bonus
projection remains eight fields per item, O(1) excluding allocated text length.
Bestiary conversion runs only on displayed rows and adds no actor scans, RNG,
pathfinding or caches. String label edits are O(L) time/output memory in rendered
text length L. Existing glyph layout/render costs are unchanged; timings unmeasured.
See revision51-stat-display-units.md for unit semantics and validation scope.
Condition label conversion uses O(S) traversal of S observed statuses and O(L)
parse/output time and memory per converted label of length L. It runs for current
revision observations; historical observation fixtures remain unchanged. No new
actor scans, pathfinding, RNG or simulation mutation.

## R51 water and hallucination motion

Hallucination samples now advance every 800 ms instead of 400 ms on all shared
presentation surfaces, O(1) time/state. Reduced motion remains frozen.
Water tilde strokes use a clipped native-resolution sine curve with fixed
per-column work and a short antialiased vertical footprint: O(native tile width
* stroke thickness), O(1) scratch, no cache or simulation/RNG access. Profile
animation speed is honored, zero/reduced motion remains static, and existing
animated-cell damage invalidation redraws the same bounded cell footprint.


### Concise jewelry descriptions (November 7, 2026)
The ring/amulet text cleanup changes only fixed catalog strings and their Pack
assembly. The 26-power catalog has no added lookup, simulation query, cache or
persistent state. For effect-key bytes E, output bytes T and player ability
entries B, one jewelry projection remains O(E + B + T) time and O(T) output
memory; the existing B scan computes the Appropriate activation discount.
Pack grouping, optional grant observation and native text wrapping/rasterization
retain their separate documented costs. Removing prose reduces text length,
not their asymptotic bounds. These are source-derived estimates, not measured
timings. Exact-copy/hover tests cover all powers and the legacy Vampirism alias;
existing capture tests cover refresh, scrolling and repeated Apply/cancel.
See [item knowledge and copy](revision50-item-knowledge.md) for the shared path.

### Revision 51 unique Composite bow generation

See [revision51-unique-composite-bow.md](revision51-unique-composite-bow.md) for the
run-wide persisted budget, all production generation routes and legacy boundary.
Normal checks add O(1) time/storage per loot draw; ordinary generation retains
O(L+E+P) for L<=20 items, exclusion bytes E and returned payload P. A defensive
missing-marker import performs one O(I+A+G+M+D) ownership/memory/stash scan, with
O(h) nested-pilot stack, no RNG, no COW detachment and no per-frame work. Existing
floor retries, save serialization and snapshot/Future costs otherwise remain.

## Revision 51 nearest-whole damage

See [damage rounding](revision51-damage-rounding.md#complexity-and-ownership).
Scalar rounding is O(1) time/space per packet or preview, after unchanged
mitigation/resistance lookups. No RNG/caching or additional world scans are
introduced. Existing owner/inventory scans, visibility, AoE, offscreen, COW and
lifecycle costs remain additive; the linked contract names inputs and bounds.

### Revision 51 fungus feasts and forward root glyphs

Victim recipes include ordinary aquatic species as dead specimens as well as
land species; this changes bounded candidate counts, not the complexity below.
Living horde habitat and setup domain/depth limits are unchanged.

See [fungus feasts](revision51-fungus-feasts.md#complexity-and-bounds) for variable
and helper definitions. Each of the three startup reservation passes includes
actual ordinary-recipe cache work, O(C*M*(Q+K+T)*J) pool selection and
O(F*B*(R*(1+P)+log B)) rootable-cell search/sort, B<=441 and R<=11. Population
adds O(F*(A+L+W+R)+V), including source scans, actor construction and a possible
root-layer allocation. Scratch/output and legacy/COW costs are detailed there.
Forward glyphs add O(1) per saved path entry to the existing O(H) expected
observation index, with no extra pathfinding, RNG or hidden-state read. These
are unmeasured source-derived bounds; new deterministic tests and coordinated
native execution are documented separately from platform verification.

### Revision 51 two-press autoexplore
See [autoexplore scope and complexity](revision51-autoexplore.md) for observation-
only frontier selection, the shared four-phase multi-goal path search, temporary
no-progress history, preserved travel/hazard dispatch, cancellation and rendering
costs, and the distinction between authored and executed/platform verification.

Revision 51 ember middle-dot repair: see [revision51-ember-glyph.md](revision51-ember-glyph.md) for authored-outline sizing, cache/native raster costs, observation safety, and fractional/HiDPI regression captures.

R51 grass and amulet display: see [glyph identity and ink placement](revision51-grass-amulet-display.md) for detached grass metadata, cached native-ink bottom alignment, unchanged background/damage geometry, appearance compatibility and validation boundaries.

R51 potion-darkness rendering: see [the rendering correction](revision51-darkness-rendering.md)
for O(D) detached footprint projection, O(V) presentation attenuation, unchanged
simulation/visibility/RNG, native pixel regressions and verification boundaries.

### Revision 51 Ichor-fish fire immunity

See [Ichor-fish immunity](revision51-ichor-fire-immunity.md#complexity-and-bounds)
for the fixed identity check, O(L) per admitted fire/ignition/hazard query,
O(C) ordered-map Burning cleanup in an existing actor pass, registry/COW costs,
shared offscreen path, observation-only prose and native layout boundaries.
No new RNG, serialized field, map traversal or cache is introduced.

Revision 51 first-four-floor primary loot: see [revision51-primary-loot.md](revision51-primary-loot.md) for seed-key hashing, fixed shuffle/count draws, bounded item allocation and diagnostics, unchanged sampler/retry costs, retained-budget isolation, save/replay paths and verification scope.

### Revision 51 water-depth presentation
See [water-depth materials, immersion interpretation, renderer parity, and complexity](revision51-water-presentation.md). The detached canonical swimming flag changes no simulation or storage; immersion transitions invalidate the whole gameplay frame and steady wet repaint remains damage-bounded.

## Revision 51 Speed-up terrain and connection presentation

Speed-up areas use their existing serialized ordered non-Euclidean assignments,
but generation replaces their cells with the `speed-up` terrain kind. Conversion
is O(K) time, O(1) additional memory for K assigned cells (at most 8×48), performed
once per generated floor. Existing bounded route/generation proofs are unchanged.
No gameplay RNG is consumed by the conversion or display.

An optional `speedUpTerrain` flag distinguishes actual r51 terrain from <=50
legacy overlays. A terrain replacement suspends a pair until every endpoint is
restored; each activity check costs O(S), S<=2 sources plus one destination.
Physical link collection remains O(G×S) time and O(G×S) output memory across at
most 384 groups. There is no cached link index to invalidate. Terrain edits use
existing floor COW mutation paths, exact lighting/optical dependency checks and
full-floor temporal/save records. No new per-turn task or expiration clock exists.

Display IDs follow serialized assignment order across random then ordered
pockets. Each lookup is O(G×S); C observed tiles cost O(C×G×S) in the current
adapter. The number string adds O(log G) bytes per drawn endpoint. Existing
native glyph raster/cache work applies, including cold multi-character raster
cost; no performance gain is claimed. Observation keeps its existing visibility
and illumination costs. Knowledge-gated previews consult observed or remembered
tile kinds, not changed hidden terrain. Additional scratch is O(E) for at most
two exits. Hidden endpoint identities do not reveal their location.

Coverage and authoritative behavior are in `revision51-speed-up-tiles.md`.

The R52 adjacent-pair clarification is a regression/documentation review of
these existing paths, not a new runtime algorithm. Generation still enumerates
at most eight fixed-length row/column candidates per eligible start, sorts the
accepted candidates and tries at most 96 complete lines per pass, including
the existing directed reachability proofs. Pair construction is O(K) plus
O(K log K) sorting for a line of K<=10 cells. No matching search, per-frame
pairing, new allocation or extra save validation was added. Number lookup,
known-map filtering and native glyph reconstruction retain the bounds above.
The new fixed-size native pixel regression performs eight full frames at two
viewport sizes; each full-frame paint includes existing observation/material,
glyph-cache and raster costs, plus O(W*H) comparison storage. It is test-only,
not a measured runtime performance claim.

### Revision 51 stale root cleanup and olive material

See [root lifecycle correction](revision51-root-cleanup.md#complexity-and-ownership).
With A floor actors, E inspected ability entries, V<=3600 cells, K links,
P saved path entries and R<=11 canonical cells, tick/load normalization is
expected O(A+E+V+P+K*R) time and O(A+V+K+R) scratch, including pair dedup,
endpoint/path checks and a full scalar-material support pass. Hash collisions
add O(A²+A*K+K²). Root severing adds O(V+K+P) cleanup per affected floor;
load/snapshot/COW costs remain additive.
No new persistent state or RNG draws. Observation retains its existing O(P)
index and native glyph/material/framebuffer costs. Bounds are source-derived;
focused tests and native before/after captures are tracked in the linked report.

### Revision 51 confirmed ability restorations

See [ability restoration bounds](revision51-ability-restorations.md#complexity-and-bounds)
for wet-cell/source cleanup O(V+P+L), per-contact O(P+L), unchanged condition
expiry/source-cache costs, floor COW/temporal boundaries and verification status.

### Revision 51 perception hover and known-only contour

See [perception UI](revision51-perception-ui.md#complexity-and-bounds) for
observation-only O(A×N + A×V×K²×R + V×B) route/light preparation (no-portal
O(A×V×R)), O(V+K+A×E) detached geometry, known-input uncertainty, pointer/paint
and retained/tile/GPU invalidation costs, native capture coverage and limitations.

## Revision 51 perception

See [revision51-perception.md](revision51-perception.md) for the pure O(1) range
and awareness primitives, per-observer effective-stat capture, scalar optics and
lighting cost, sleeping-cap route work, unchanged cache invalidation, AI cadence,
versioned heart rewards and focused regression coverage. Desktop known-safe
contours have their own [perception UI ledger](revision51-perception-ui.md).

## Revision 51 aiming delivery and impacts

See [aiming paths](revision51-aiming-paths.md#complexity-and-bounds) for shared
physical-ray policy, observed-only preview costs, portal/actor/clone work,
Blink admission, Beckoning candidate routes, O(D) detached invalidation, native
raster/GPU work, and verification limits. Iceblast delivery remains a designer
choice; existing radial/mental/conjuration behavior is not silently changed.

## R51 displayed-sRGB illumination calibration

See [illumination display calibration](revision51-illumination-rendering.md) for
fixed-knot smooth interpolation/tails, separate source hue, O(V+F) hue buffers,
O(C+U+A) final detached-cell grading, retained-cell storage, animation restoration,
and native/full/retained/tile/GPU verification costs. Simulation and visibility
algorithms are unchanged; old v48 rendering remains an explicit frozen adapter.

## Revision 52 exact Darkness illumination and radius-3 circle

See [Darkness override](revision52-darkness.md) for bounded sparse masks,
scalar/cache/observation costs, additional render scratch, revision identity,
source-preserving expiry, and deterministic test boundaries. New application
scans 49 offsets and retains at most 29 circle cells, with O(49 + 29D) time and
O(29) scratch for D unique stored mask entries; legacy squares remain unchanged. This change adds
no propagation, pathfinding, gameplay RNG or animation-owned simulation clock.

### Revision52 enemy outcome cues

See [enemy animation matrix](revision52-enemy-animation-matrix.md). Presentation-only classification is O(T) per event for T authored traits. Direct scalar-transition hooks are O(1) plus existing visibility/plane admission; exact area cues cost O(C × V) time and O(C) detached output for C already-resolved cells and visibility-query cost V. Slime adds at most nine delta cells per actual move. Flood cues reuse the new frontier instead of rescanning a floor. Fungal roots and breath reuse their existing resolved paths/footprints. No added gameplay search, RNG, persistent state or wall-clock simulation is introduced. Native painting cost remains O(number of visible cue cells); bounded batches share the central presentation cap. Timings are not yet measured.

## Revision 52 player outcome animation

See [player animation bounds](revision52-player-animation-complexity.md): detached observed-actor command snapshots, direct outcome hooks, exact footprint costs, pursuit-route helper cost, and unmeasured visibility/cache assumptions. No simulation clocks or saved animation state.

## Revision 52 admission and animation preferences

See [admission and settings](revision52-admission-and-settings.md) for exact revision gates, unchanged inherited numeric profiles, bounded Settings choices, constant animation-mode draft controls and existing O(J) atomic preference persistence. Cosmetic timing is outside the saved simulation and RNG.

## Revision 52 inventory-only outcome feedback

See [inventory animation cues](revision52-inventory-animation-cues.md) for actual-use and payment admission, O(I+B) existing inventory scans, O(I) changed-reserve scratch, constant detached cue emission and strict separation from gameplay RNG/save state.

## Revision52 action animation

See [action animation contract](revision52-action-animation-contract.md) for detached event/route bounds, per-action timing, native raster cost, observation snapshots and verification limits.

## Revision 52 natural Fighter birth profiles

See [Fighter balance](revision52-fighter-balance.md) for the bounded150-row, exact-revision lookup at new natural birth only, one-time embedded JSON parsing, O(NR) additional floor-generation work, preserved RNG/rewards and strict separation from loaded, copied and summoned actors. Model validation is distinct from integrated-engine tests.


## Revision52 reachable targeting and Sparkle

See revision52-targeting-sparkle.md for bounded portal planning/traversal,
O(V*(AP+M+G)) Blink landing filtering, observation/render costs and unchanged
permanent-source/water cleanup costs. Bounds are source-derived; no new
retained state or timers are added by these changes.


## Revision52 enemy follow-ups

See revision52-enemy-followups.md#complexity for the complete bounds and
unmeasured profiling limits. Wound settlement is O(C + D*P), with at most three
future buckets per recent source and normal-upkeep-only execution. Admission
adds O(sum C) across persisted actors and carried pilots, plus the documented
nested serialized-value scan at storage boundaries. Dragon minimax routing is
O((V+E+K) log V + V*H + E*T + A*I), with E <= 8V, V <= 3600, and at most two
route fields per action. No route cache hides invalidation or movement work.


## Revision 52 amulet tier desktop presentation

See [amulet tier presentation](revision52-amulet-tier-ui.md) for pure shared
profile lookup, typed physical-instance grouping, existing O(I²) equality/rank
work and O(G log G) display sorting, O(T) text construction, bounded DEBUG row
painting and full/retained/tile verification. Shared interaction damage bounds
add fixed two-logical-pixel plus one-native-pixel padding per side: O(K)
footprint construction and queued rectangles for K old/new interaction cells.
Existing span, raster and upload costs remain area/candidate-dependent. No
pixel-diff invalidation or forced full repaint was added. There is no duplicated
tier table, simulation mutation, RNG, new visibility query or per-frame tier
cache. Native performance remains unmeasured.


## Revision52 immutable amulet tier mechanics

See [amulet tier complexity](revision52-amulet-tiers.md#complexity) for fixed nine-row profile lookup, one-time birth stamping, persisted-item and captured-rate validation, temporal boundaries, and bounded utility-range changes. No new RNG, path search, target count, item identity or recharge operation is introduced. Source-derived bounds are separate from measured runtime evidence.

### R52 all-role contextual birth profiles

The finalized all-role selection expands the fixed birth catalog from 150 to 550 rows. For R=550 rows, each new natural/Nest/summoned actor performs an O(R) exact-context nearest-floor lookup after legacy overlays, plus O(1) stat and optional saved-coefficient writes. There is no action-time depth scaling or RNG draw. Saved heal/special-damage reads are expected O(1); added condition validation is O(K) within each actor's existing condition traversal, and save-schema recursion remains O(J) serialized bytes. Catalog parsing/storage is O(catalog bytes) once; actor coefficient storage is O(1). Global gas/fire simulation, pathfinding and rendering costs are unchanged. See `revision52-all-role-balance.md` for scope, assumptions, limits and verification boundaries. These bounds are not timing measurements; profile full-world generation before introducing a lookup cache.

## Revision52 Bestiary birth-stat ranges

See [Bestiary stat presentation](revision52-bestiary-stats.md#complexity) for
bounded public birth reconstruction, exact-profile/legacy-recipe floor lookup,
revision-keyed cold fungus-feast admission caches, native font-fit/raster costs
and unchanged discovery storage. The warm range reducer is
O(R*J + D*(A + (T+R)*J + S)), with D <= 40, S = 7 and R = 550; recipe construction
and cold feast/helper work are separately accounted for. No hidden Run actors,
RNG, gameplay mutation, new selector or per-actor cache is added. These are
source-derived bounds, not measured native or Windows performance.

## Revision 52 Floods shallow-to-deep conversion

See [Floods complexity](revision52-floods.md#complexity) for the bounded 15-cell
geometry, original-shallow snapshot, O(C*(T+M+P+G+B)) guarded ledger merge,
single-record storage, inherited floor copy-on-write and normal/offscreen expiry,
visibility/cache costs, unchanged damage/push work and detached animation cells.
No new per-frame simulation, RNG, terrain stack or native rendering path is
introduced. Bounds are analytical; focused/native verification is distinct.
The 2026-11-08 source reconstruction retains these bounds. Its explicit
promotion helper rechecks the shared protection gates before merging each
record and does not introduce a separate terrain stack. Reconstruction tests
and provenance are recorded in `revision52-floods-reconstruction.md`.

## R52 ground-marker presentation
See [ground-marker presentation follow-up](r52-ground-markers.md) for native antialiased arrow cost, lowered small topology numbers, and removal of player linked aliases. No simulation complexity changes.

## Revision 52 Minotaur ordering and collapse shape

See [movement follow-up](revision52-charge-collapse.md) for the unchanged charge
step budget, deferred haste selection costs, alternating cardinal/diagonal-inclusive
collapse frontier and per-floor serialized phase. No per-frame simulation or RNG
is introduced. Bounds are analytical, not measured performance.

## R52 Weave of Time arrival consumption

See [Weave arrival-consumption complexity](revision52-weave-consumption.md#complexity)
for expected constant endpoint removal, linear middle removal, prefix-deletion
cost, retained allocation, candidate clones and shared-trail detachment. No
per-frame simulation or new RNG draw is introduced.

## R52 negative illumination and Brogue chasm
See [negative illumination and chasm presentation](r52-negative-illumination.md): fixed role curves/contrast and static cosmetic hashes, O(C+A+U) detached preparation, O(C) cached metadata, no new world queries.

## R52 ratio accuracy and birth baselines

See [ratio accuracy checkpoint](revision52-accuracy-formula.md#complexity).
The shared revision-aware base chance uses O(1) time and extra memory for fixed
numeric inputs, retaining the existing range exponentiation. It adds no RNG,
allocation, cache or world query; effective-stat/illumination/attack/render work
retains its existing bounds. New-body and player Accuracy/Dodge assignments
add O(1) work at existing birth boundaries, with no saved marker or retroactive
traversal. Historical balance/release receipts do not certify this change.

## Visible ground sidebar and behavior labels

See [visible-ground-list.md](visible-ground-list.md) for observation-time admission,
G-entry category/distance sort, measured card wrapping, retained/tile repaint,
and input-path bounds. Ground collection reuses existing visibility; presentation
adds no simulation dispatch or animation-time world lookup. Restored monster
state labels add O(label bytes) capitalization and measured wrapping per card.

## R52 fixed Dodge, innate damage and carrying follow-up

See [stat follow-up complexity](revision52-stat-followups.md#complexity).
First-appearance Dodge adds an O(R*L) fixed-catalog scan only at birth (R=550,
L=key bytes), with O(1) scratch after the shared catalog load. Innate arithmetic
and Jackal heartless writes are constant. Revision-aware carrying checks retain
O(I+B) category admission and existing jewelry-removal clone/ability work.

Brogue-style gameplay animation: see [revision52-brogue-animation-style.md](revision52-brogue-animation-style.md) for two rendering primitives, quiet ordinary actions, exact route projection, local transition feedback, native raster cost and verification scope.



### Chasm contrast correction (November 2026)
See [r52-chasm-color-correction.md](r52-chasm-color-correction.md). For C detached terrain cells, finishing remains O(C + A + U), with existing animation anchors A and underlays U. A fixed set of default-profile checks is evaluated once per terrain rebuild; each existing boundary (at most four per cell) uses fixed RGB math. No extra cell scans, world/visibility reads, cache, RNG or retained storage are added. The historical bare-chasm skip of the bounded 12-step contrast search is superseded by the [ordinary illumination clarification](revision52-normal-chasm-illumination.md); visible chasms now share that same bounded search with other terrain. Extra memory is O(1); glyph rasterization/cache misses, pixel coverage and full/retained/tile costs are unchanged. These are analytic bounds, not benchmark claims.



## R52 reserve ceiling and Wait recovery (2026-11-08)

See [the reserve normalization and payment ledger](revision52-reserves-wait.md#complexity).
Boundary normalization scans all actor/reserve/item projections in O(A+B+I+K),
with O(D) recursive-pilot stack and full payload COW copies only for changed blocks.
Acquisition retains O(B) reserve validation/clamping; payment/UI/Reaping/Wait add
constant arithmetic to existing lookups/scans. No per-tick migration or new RNG.


## Revision 52 Hanged damage redirection

See [Hanged](revision52-hanged.md). Per incoming packet, O(1) scalar conversion
follows existing mitigation. Ownership/ability inspection adds O(A+K); actual
resource-history visibility/stat work and shared-body synchronization retain
existing inclusive costs and COW detach behavior. No per-frame scan, RNG, saved
state or additional energy pool. Load normalization scans persisted actor
metadata (including initial player and nested pilots) once per normalized world;
nested Future/history/pending states retain existing migration traversal costs.
First-use versioned catalog construction clones fixed catalog/text once; warm
access is O(1). Focused tests and source estimates are separate from full-candidate
verification and measured profiling.


## R52 native glyph coverage
See [glyph mask coverage correction](revision52-glyph-mask-coverage.md) for fourfold source sampling, final alpha-area integration, bounded cached masks, cold raster cost, retained/GPU behavior and native verification scope.



### R52 desktop source reconstruction (2026-11-08)
The reconstructed Bestiary revision handoff and species damage conversion are
O(1) time/O(1) extra memory per call. Existing birth/catalog/font costs are
unchanged. The reconstructed Floods CPU oracle covers 58 frames and adds
O(F*P) exact comparison time with O(P) buffers and O(F*C) detached views, plus
production observation, Scene/tile planning, effect projection, glyph raster
and PNG-encoding work. See [the reconstruction record](revision52-desktop-source-restoration.md)
for variables, bounded fixtures, checks and source-provenance limits.

## R52 source and delivery provenance update

The Windows packager's snapshot and archive stages retain O(B + N log N) hashing
and sorting for B source/artifact bytes and N files. Additional design reports
are bounded file inputs. Complete source archives include fixtures and authoring
assets rather than relying on missing intermediate deltas; compression work is
linear in bytes for a fixed compressor setting, with bounded streaming buffers.
These delivery operations are offline and never run per game frame or turn.

## R52 Avatar selection and fixed player glyph

See [new-game boundary](avatar-new-run-boundary-r52.md),
[menu selection](revision52-avatar-menu.md), and
[player glyph presentation](player-glyph-presentation.md). New Avatar wrappers
add constant boundary work while preserving full generation costs. Seed editing
and selection remain bounded by the existing seed-length limit; native menu
painting includes glyph raster/cache and pixel costs. Player glyph projection
adds constant work at the existing observation boundary and does not rewrite
saved actors or appearance profiles.

## R52 monster status and gas-entry presentation

See [monster status presentation](monster-status-presentation.md) for actual
capability lookup, condition formatting/wrapping, public-template projection,
retained-cache and native painting costs. See [gas-entry warnings](gas-entry-warnings.md)
for the additional constant-time current-cell gas-presence query and unchanged
movement/hazard planning costs. Neither feature adds per-turn simulation work.

## R52 incremental corpse roots

See [root growth](revision52-root-growth.md) for per-link cadence, visible-prefix
projection, corpse search/path costs, layer allocation, legacy-save compatibility,
normal/offscreen scheduling and native rendering limits. Analytical bounds are
separate from executed regressions and are not performance guarantees.

## R52 complete source evidence

Source snapshots now also include all documentation and third-party notices,
with deduplicated paths. For N source paths and B bytes, the snapshot costs
O(B + N log N) time and O(N) path/hash metadata, with streaming file reads.
This catches missing or modified licensing/documentation before packaging without
adding game-frame or simulation work.

## R52 Fated Duel lifecycle

See [Duel termination](revision52-fated-duel.md) for indexed attack/upkeep
validation, conservative independent-owner candidates, event normalization,
structural reconciliation, temporal restore and COW costs. Clean movement checks
scale with candidate owners rather than all world actors; an adversarial world
with many independent owners can still have quadratic event work. No serialized
index, RNG, altered area or resource-cost rule is added.

## R52 lake currents and island safeguards

See [square currents and lookahead](revision52-square-currents.md) for bounded
coverage, seeded edge/collision decisions, lake component scans and deferred
floor-generation costs. See [Swimming island safeguards](revision52-swimming-islands.md)
for water/flight ownership queries, land-component predicates, bounded two-label
ordinary/assisted escape reachability, four-state ring-acquisition/entrance progress, deterministic item
placement and bounded candidate retries.
One-floor speculative lookahead reduces eager generation; explicit requests and
real offscreen fall destinations can still generate deeper required floors.
Generation work and static escape validation are not frame-time or universal
gameplay-safety guarantees.


## R52 enclosed enemy perception preview

See [enemy perception overlay](revision52-enemy-perception-overlay.md) for detached
knowledge bounds, remembered-light uncertainty, complete contour extraction,
map-edge clipping, damage bounds and native raster costs. This is observation
and presentation work; it does not change simulation perception, reveal unknown
terrain or run gameplay randomness.
The R52 follow-up adds O(B) known-special-sense classification per creature,
O(1) all-known/radius checks per candidate and at most one additional detached
route query for active Dream See; the existing ray/portal asymptotic bounds
remain unchanged. Full known-cell and source-bound input/raster regressions are
specified in that linked entry; their live-world oracle is test-only.


## R52 dark chasm memory presentation

See [dark chasm memory](revision52-dark-chasm-memory.md) for the narrow color
projection rule, ordering with illumination/memory contrast, raster work and
unchanged simulation/visibility costs.


## R52 corpse colors and cross

See [corpse colors and cross](revision52-corpse-colors-cross.md) for retained color
projection, existing saved-color compatibility and independent marker raster
costs. No serialized field or gameplay rule is changed by the visual presentation.


## R52 shallow-water automatic pathing

See [shallow-water pathing](revision52-shallow-water-pathing.md) for the shared
known-terrain admission predicate, route-search and execution validation costs.
Manual movement and static island escape searches retain their rules.


## R52 wall and stale-flesh color distinction

See [wall and stale-flesh colors](revision52-wall-flesh-colors.md) for material
classification, palette projection and native rendering costs, including the
player-knowledge boundary. No terrain or sight rule changes are implied.


## R52 wall-feeding state presentation

See [feeding state](revision52-feeding-state.md) for the actual feeding signal,
projection frequency, display costs and knowledge boundaries.


## R52 Fast and Haste action scheduling

See [Fast versus Haste](revision52-fast-versus-haste.md) for bounded action passes,
AI/visibility/pathfinding work per pass, movement-success guards, suppression and
presentation costs. The extra decisions must include their existing AI costs,
not be described as constant-time simply because the action count is bounded.

## R52 corpse-cross glyph centering

See [native ink centering](revision52-corpse-cross-centering.md). The marker now
uses the actual base glyph's un-clipped native ink bounds, returned from the same
paint call. Cold ASCII metadata adds O(H + R) work from existing row runs;
compound/fallback metadata adds O(M) final-mask scanning, with O(1) bounds per
bounded cache entry. Warm attachment adds O(1) work without a second raster,
framebuffer scan or world query. Existing marker O(P × 16) raster/command costs,
color/alpha, clipping, outlines, save format and privacy rules remain unchanged.

## R52 exact-zero health and current-HP labels

See [exact-zero health](revision52-zero-health.md). New R52 boundary settlement
adds O(A) scan/O(Z) transient candidate IDs plus the existing death/rescue/visibility,
loot and copy-on-write costs. Offscreen admission adds O(A) scan/O(1) memory before
existing precise simulation when unresolved deaths exist. HP display conversion
is O(1) per label; actual state, bars, RNG and historical revisions are preserved.


## R52 ordinary chasm illumination clarification

See [ordinary chasm illumination](revision52-normal-chasm-illumination.md).
Removing the chasm-specific negative-light contrast skip leaves the detached
finishing stage O(C + A + U) per rebuild, for C cells, A animation anchors and U
underlays. Each visible chasm now pays the same fixed RGB arithmetic and bounded
12-step contrast-gain search as ordinary terrain. Extra scratch is O(1); no new
stored fields, cache, world reads, visibility work, allocations, glyph masks or
raster passes are added. Native draw cost and cold glyph-cache work are unchanged.
These are analytic bounds, not measured timing claims.

## Revision 52 released Floods split

The radial proposal is deferred. Native Apply and recorded UseAt commands retain
the released wave algorithm and its at-most-15-cell footprint. The existing
[wave complexity](revision52-floods.md#complexity) remains current; no radial
49-cell pass, current-grid allocation or O(V) current restoration scan executes.
Temporary-terrain Snapshot validation retains O(sum(T_f)) record traversal over
stored floors with existing terrain validation. The removed module's historical
bounds remain in the explicitly [deferred proposal](revision52-radial-floods.md#complexity).
Test-only replay checks use existing whole-state storage/replay costs; they add
no production per-command or rendering work.

Revision 52 flying chasm pursuit: see [revision52-flying-chasm-pursuit.md](revision52-flying-chasm-pursuit.md) for departure detection/actor lookup bounds, unchanged normal pursuit and nine-cell transfer costs, explicit saved chase flag, finite-flight/Dragon behavior, COW/temporal costs and deterministic verification scope.

## Revision 52 corpse current physics

See [corpse currents and fallen flyers](revision52-corpse-currents.md#complexity)
for stable-ID corpse occupancy snapshots, per-normal-turn sorting, exact
inactive-floor activity, command/phase-boundary gravity scans, rooted-body predicates, root-sever/COW/index costs,
constant-time dead-flight eligibility, and the unchanged placement/fall costs.
No new rendering, save schema, RNG or pathfinding pass is introduced.


## R52 minimum starting creature Dodge

See [birth-only minimum complexity](revision52-birth-dodge-minimum.md#complexity).
The minimum adds O(1) scalar comparison time and O(1) scratch at existing births.
The O(R*L) first-appearance lookup, fixed tuning scans and shared catalog parse
remain unchanged. No per-attack, per-frame, save/load or copy/revival traversal,
allocation, cache or RNG work is added. Bounds are analytical.

## Bestiary nearby-effect range prose

See [effect range audit](revision52-bestiary-effect-ranges.md#complexity). The
full-page-only borrowed text adapter adds O(M*L) fixed-dispatch comparisons for
M mechanics and ID/name length L, plus the existing bounded O(C) text layout
and rasterization for C selected-page characters. It does not query a Run, alter
preview prose, or add any simulation/pathfinding work. Native art/font cold-cache
and framebuffer costs remain in the existing Bestiary renderer, not hidden as
constant-time simulation work.

## Never Surrender DR eligibility and preview parity (2026-11-09)

See [the DR-source, preview-admission and Hanged-payment audit](revision52-never-surrender-audit.md) for the new bounded source-ability check, reserve lookup costs and conditional two-endpoint stat queries. Hanged and ordered rescue implementation are unchanged.

## Revision 52 empty-direction amulet input

See [directional zap input](revision52-directional-zaps.md#complexity) for the
first-key player-relative origin correction, unchanged mouse/Enter admission,
O(I) carried-item classification and retained O(V) projected-cell confirmation.
No new key bindings, simulation loops, state fields, visibility queries or RNG
choices are introduced. Existing preview, launch and native rendering costs are
unchanged; the focused helper/capture suite is not OS input-delivery evidence.

## Revision 52 player resource turn changes

See [player per-turn resource display](revision52-resource-changes.md) for the signed net/current-maximum convention, baseline storage and cache invalidation, legacy/save/temporal boundaries, and inclusive complexity.


## Revision 52 menu-entry god randomization

See [menu visit complexity](revision52-menu-god-randomization.md#complexity).
The entry observer adds O(1) work and storage per queued-input/frame boundary;
only an actual entry reads 12 OS-random bytes and runs the fixed 12-element,
three-swap selection, O(K + S) time/O(K + S) scratch for K=12 gods and S seed
bytes. It does not query a Run, parse/generate a floor, consume gameplay RNG,
or touch a save/replay. A changed full prefix later uses the existing O(S + A)
seed-sync parser once, including catalog initialization and starter construction.
Native font/raster/cache/upload costs are unchanged. These are analytical bounds,
not measured frame timings; OS entropy service latency is platform-dependent.

## Revision 52 experimental Fast mode

See [effective-depth routing, sparse identity and generation](revision52-fast-mode.md#complexity),
[heart reward arithmetic](fast-mode-heart-rewards.md#complexity), and
[menu, HUD and archive rendering](fast-mode-menu.md#complexity). Downward routing and
mode checks are O(1); upward/sparse gas lookup is O(F), F<=40. Existing generation,
travel, placement, visibility, COW and serialization costs remain explicit in the
subsystem entry. There are no skipped-floor payloads or eager replay file scans.
Only the heart's normally awarded stats and blessings receive compensation.
Initial-floor/finale design and complete production/desktop validation are still
open; these bounds are not measured speedup or release-readiness claims.


## Revision 52 magic and physical projectile travel

See [the travel amendment](revision52-magic-projectile-travel.md#complexity) for per-visible-cell timing, explicit delivery metadata, staged observed Firebolt terrain, new-command simulation compatibility, live portal traversal, and native raster/invalidation costs. No wall-clock animation state enters a Run.

## Revision 52 staged Firebolt terrain

See [Firebolt terrain presentation](revision52-firebolt-staging.md#complexity) for
command-only outgoing observation costs, bounded detached terrain masking,
arrival-driven native full damage, custom-light reconstruction, canonical actor
and knowledge preservation, and replay autoplay presentation pacing. Simulation
and saved/replayed command timing remain independent of animation.

## Flush three-column HUD layout (November 13 UI mockup)

Presentation only: Chronicle/map left, observed entities middle, player resources
and abilities right, with boxed player stats at the bottom. Geometry resolution
is O(1) per frame and input event, O(1) extra memory. Shared panel borders add a
constant number of rectangle commands; software raster work is proportional to
the clipped native edge pixels. Ability name wrapping uses existing measured
word wrapping: O(T²) worst-case font metric work and O(T) temporary storage for
T characters per admitted name, at most two painted lines per visible row.
Entity content retains the existing bounded observation-only wrapping/sorting
contract. Panel invalidation now targets the right-middle column in both
retained and tiled renderers; no simulation or visibility calculation changes.
Verify with enemy_list, quiet_ui, responsive_geometry, retained/tile damage and
native generated_mid_depth_screenshot tests. Bounds are responsive logical
coordinates transformed at the native drawable scale; costs are unmeasured
asymptotic estimates, not new benchmark claims.

### In-bar resource labels
Player and observed-creature resource meters are 24 logical pixels thick, with
labels and totals inside. Two text passes clipped at the fill boundary select
contrasting ink for filled and empty regions. Per meter: O(G) metric/glyph work
and O(W*H) software fill pixels, O(G) fitting storage; native GPU quads constant
and glyph commands O(G). Caller clips and damage masks remain authoritative.
No resource values or mechanics change. Validate zero/partial/full meters and
retained/tiled resource updates with resource_ui and enemy_list tests.

### Upcoming energy upkeep forecast
Read-only projection follows canonical active order, atomic payments, own-reserve
priority, next-normal-turn prepayment credits and remembered-grant expiry.
Uses O(B*L) copied reserve metadata, expected O(B+A*L) lookup/planning for B
reserves, A active abilities and L ID bytes. Unspeak additionally scans current
floor actors and projects one cloned target's traits/catalog costs. No Run clone,
RNG, turn advancement or live state mutation. It forecasts maintenance with
current state, not uncertain combat procs, future commands or energy recovery.
Resource hatch raster is O(segmentWidth*barHeight), O(1) extra memory; text
contrast clips in three segments with O(G) glyph work. Display delta keys include
the projected debit in both retained and tile renderers.
Creature cards now place resources directly after the name, state/conditions
below those bars, and spacing after the complete group. This is O(1) geometry
per existing line; content collection, hit testing and scrolling remain shared.

## Compact ability meters and linked creature hover (2026-11-14)
- Ability reserve/upkeep presentation is read-only. One shared projection per observed ability list; canonical atomic payment plans split own reserve from player spill. No live simulation mutation or random draws.
- Ability meter names wrap in their existing bounded row; state is underline/circle rather than added prose. Meter raster work remains bounded by the clipped HUD area; diagonal hatching is O(segment area).
- Monster conditions retain numerical durations, use inter-card space first, and expand only with wrapped content. Shared layout computes painting and hit bounds, including staircase prefix rows.
- Regression coverage exercises both hover directions for multiple actors and a wrapped long name across three window sizes, requiring identical highlighted frames for list and map pointer selection.

## Text-fitted ability bar height (2026-11-14)
- O(G) width measurement chooses one-line name/count when it fits; otherwise existing measured wrapping places the count under the full name. Height derives from rendered lines, font size, and padding rather than a fixed tall rectangle.
- Four window sizes and representative longest ability names check enclosure, unchanged name text, row bounds, and no drawing beyond bar height. Existing resource and forecast repaint tests remain applicable.

## Boundless Hunger display scale (2026-11-14)
- Preserve uncapped gameplay energy and X/∞ display. Both fill and projected cost magnitude use a fixed100-point visual scale; fill and hatching saturate within the existing bar rectangle. Hatching denotes cost, not resulting bar shrinkage.
- Readonly canonical upkeep still spends own reserves first. Tests cover250/110/90 energy,20 upkeep,5 reserve/15 player spill,150-cost saturation, and label enclosure at three window sizes. No overflow graphic or gameplay rule added.

## Stroke-only upcoming-cost warning (2026-11-14)
- Upcoming-cost warnings preserve the resource's original fill. Only the boundary and diagonal strokes are indigo; base fill and text contrast remain unchanged for player and divine reserve meters.
- Raster complexity remains O(hatched area), with no additional overlay allocation. Tests sample player-blue and two god-colored reserve bars to prove pixels between strokes are unchanged.

## Unified blue reserve fill (2026-11-14)
- Divine reserve meters now use the same configured bright energy-blue fill as the player meter. Previously their god colors remained underneath stroke-only warning hatches; the magenta reserve fill could be mistaken for warning tint.
- Indigo strokes use RGB110,90,235, distinct from dark text; there is still no warning background fill. Render order is base fill, warning strokes, then readable text.
- Pixel comparison of actual native screenshots checks untouched blue between warning strokes against an adjacent ordinary blue region rather than relying solely on helper unit tests.

## Adjacent one-third reserve bars and Ragelight presentation (2026-11-14)
- Reserve meters occupy one third of the prior rail width, right-aligned beside the ability name; names wrap in the remaining space and active names remain underlined. Numeric NRG and indigo cost warnings remain inside the blue meter.
- Ragelight is a presentation alias, leaving serialized ability IDs and canonical names used by simulation dispatch intact. Detached UI and final text measurement/rendering use the same alias so legacy log text is also shown correctly without changing saves.
- O(T) alias lookup uses borrowed text when absent; wrapping/hit geometry retains the existing bounded rows. Four window sizes verify name/count enclosure and exact one-third sizing.

### Ability enable indicators (2026-11-15)
Each displayed non-passive ability uses a fixed 11×11 circle to show enabled state, including reserve-bearing rows; no underline. Per redraw O(A + G) indicator/text-layout work for A visible rows and G glyphs, with A bounded by viewport capacity, plus existing glyph rasterization costs. Indicator adds O(1) memory per row; no simulation changes.

### Dedicated inspection strip (2026-11-15)
The bottom inspection line owns a font-sized strip between the main panels and toolbar, with shared borders and inner clipping. Height depends only on the fixed typography profile, so hover changes do not reflow the map. O(G log G) worst-case text fitting, O(G) text paint and O(W×H) software strip fill for G text glyphs, W strip width and H bounded line height; constant panel geometry and O(G) temporary fitted string. Both retained and tile damage cover the full strip when notice changes.

### Compact ability margins (2026-11-15)
Ability row spacing halves the unused gap around a 19px reserve meter (50px to 35px at default typography), preserving the existing minimum for enlarged fonts. O(1) geometry; drawing/input share the same row and capacity functions. At most viewport-height/row visible entries; no new allocations or gameplay change.

Fortune Teller presents three fixed boxes and the single next global 20-turn refresh countdown in the first empty box. Detached O(1) projection and constant paint primitives; countdown/charge changes are included in both retained damage keys. No simulation timing change.

Amulet reserve bars read existing physical item energy/capacity during inventory/visible-ground observation. O(1) additional projection and fixed rectangle paint per amulet; sidebar names wrap in remaining width, pack names use existing measured fit. No extra hidden-item reads or simulation mutation.

Execute revision52 per-proc debit is10 via the existing atomic reserve-first planner; revision51 and earlier retain20. O(1) cost selection, existing payment complexity unchanged. UI reads that same cost function and uses the exact authored description. Ragelight copy change is presentation-only.

### Float Away direct-damage boundary
R52 Float Away now uses the existing direct-packet flag: gas, lingering fire and periodic status damage cannot grant its flight or displacement. Melee, immediate ranged spells and other existing damage kinds retain the proc; other after-damage effects and pre-R52 replay behavior are unchanged. Regression tests cover nine indirect kinds and five direct kinds.

### Telepathy hallucination override
R52 sensed actors skip only hallucination identity/cache projection; ordinary disguises and pre-R52 presentation remain unchanged. Per admitted actor, the existing telepathy query scans body origins/abilities and can perform Dream See optical LOS (A actors, B abilities/origins, R ray/portal routing cost); worst-case additional work is the existing sense-query cost per presentation surface, with O(B) origin storage. Inspection uses its existing hallucination-cache result to avoid a second sense scan. No RNG, simulation, range, shared-viewpoint, or line-of-effect changes. Regression tests cover global and target-specific telepathy, six animation phases, map/list/inspect/log identity, and dark Iceblast targets with wall/creature blockers.

### Target-selection ability orb
The existing pending ability is projected to a detached selected-ability ID at the pointer synchronization boundary. Its orb is orange while selecting, with orange taking precedence over enabled green; non-targeted passives remain undecorated. Both retained HUD keys include this ID so cancellation restores normal ink without rebuilding world observation. Regression covers targeting entry/cancel and both retained painters at three window sizes.

### R52 Half Here placement turn (2026-11-15)
Current placements spend 5 energy (own reserve first) after target and identity validation, with no placement lock. Dismissal is free immediately. The payment uses an expected O(L) reserve lookup for an L-byte ID and the existing bounded identity preflight and resource telemetry; rejection does not spend energy or a turn. Older command prefixes still use their original placement rules.

Live R52 placements store one deadline at placement tick + 5. Until then, dismissal, replacement, swapping, and forced displacement cannot move the clone. Free inputs and Haste bonus phases do not advance the clock. Death, forced hazard cleanup, and floor changes still remove endpoints normally. The deadline adds expected O(1) work per relevant action and O(1) saved state, with no per-turn scans or cache. During the lock a player step adds one actor lookup and O(B * P) portal-alias checks, where B is the number of candidate exits and P is the existing alias-query cost. A command-index policy preserves historical replays and journals.

Successful placement enters the existing normal action scheduler; dismissal stays free and invalid placements report no action. The free-command gate and activation result each scan active ability IDs (A): O(A) time, O(1) extra memory per command, no RNG or new cache. Existing endpoint validation/creation and turn scheduling retain their prior costs; no new traversal is introduced. Verify successful placement, invalid/missing targets, dismissal, haste phase behavior, and pre-R52 fixture compatibility with focused Half Here tests.

### R52 Composite bow accuracy

Each shot/forecast checks the revision and bow kind once, O(1) time and memory.
The resolver passes an explicit accuracy-only flag through deflection recursion;
it adds no scans, RNG or persistent state. Effective-stat, collision/trajectory,
rendering and existing recursive combat costs remain unchanged. Only hit-chance
range becomes zero; physical range and ranged admission stay intact.

### Death acknowledgement gate
A constant-size desktop input gate blocks gameplay interaction until a dead-run frame has been presented and a fresh Space press arrives. Held or queued Space cannot skip the death view. Terminal auto-travel/rest/targeting are cleared, and dispatch refuses commands on dead runs without mutating save/replay input. Window/quit events remain live; archival and autosave paths are unchanged. Tests cover input gating, release/repress, new-run reset and terminal run immutability.

### Surprise response timing
R52 successful player hits on sleeping/wandering targets record an internal global-tick boundary before waking them. Their normal/bonus response and immediate counter-attack are blocked through that attack's AI phase, then normal behavior resumes. Direct spell damage uses the same pre-hit snapshot; misses and periodic status damage do not mark a target. Other creatures and historical revisions are unaffected. Dead actors are excluded so corpse lifecycle stays unchanged.

### Fortune Teller relative refill schedule (R52)
One keyed `turn_schedule::Schedule<()>` deadline per actor with depleted fortunes. Spending schedules now+20 only if no event exists; due events add one and reschedule by20 until cap3. The existing upkeep actor loop queries a constant-sized queue; no additional actor scan, periodic modulo polling, or decrementing countdown is introduced. UI computes remaining turns from the stored deadline. Save/clone preserves queue; older depleted R52 saves begin one fresh20 interval on first upkeep, while pre-R52 retains clock-aligned behavior. Due catch-up is bounded by three missing charges.

### Cogworks modes and rally marker
Cogworks projects its canonical mode to three compact clickable bubbles, with only the selected mode filled (orange during targeting, green otherwise). Existing numeric mode commands remain valid and both retained HUD keys include mode changes. Rally marks only an already knowledge-projected tile: standalone flag on terrain, superscript appended to the highest-priority entity without replacing its glyph/rank; animated terrain rebuilds retain standalone flags. Three-size native render tests compare retained paths and all mode transitions.

### Tinkered jewelry display names
The shared R50+ item-name projection appends the one Tinker grant to canonical ring/amulet names, retaining amulet tiers and using the current Ragelight alias. Existing base grants are not duplicated; Tinker's last added grant supplies the suffix. Stored item name, ID, grants and effects remain unchanged. Pack, ground/inspect, pickup/drop and Tinker log callers already share this projection. Tests cover tier preservation, old revision behavior, read-only state and duplicate prevention.

## Transient actor status-death upkeep guard (2026-11-15)
After status packets execute, normal upkeep revalidates its snapshotted actor ID before regeneration. Mini-Me may be physically removed by lethal venom/bleed; missing or dead actors skip the remaining upkeep. This adds one existing actor lookup per processed actor (same lookup/index assumptions as the adjacent upkeep operations), O(1) extra storage, no queue or scan beyond the existing lookup. Deterministic regressions cover wait/move, loaded pre-death state, lethal bleed and surviving venom. This prevents an Option unwrap panic; Windows runtime remains separately unverified.

### Amulet insufficient-energy copy
One shared admission-message formatter maps only InsufficientPlayerEnergy to the exact player-visible text "Insufficient Energy". Pack activation blockers and core target admission use it; other diagnostics and all resource mechanics remain unchanged. Core copy and actual Pack blocker regression tests cover the two paths.

### Health threshold acknowledgement
A constant-size desktop gate tracks resolved HP downward crossings at40% and25%. The lower crossed threshold wins if one action crosses both. It re-arms by observing healing above each threshold, does not warn on initial low-health loads or max-HP-only changes, and gives terminal death precedence. Only a fresh Escape after the warning is presented dismisses it. Automatic travel/rest and targeting are interrupted; simulation/save state is not altered. The warning forces safe overlay uploads and invalidates retained state on entry/exit; native screenshot and input/threshold tests cover the flow.

### Text audit batch two
Removed only the requested redundant clauses for Cleansing, Healing/Energy potions, Quick Hand, Dreamwalking, Blood Weapon and Nova; changed All-Seeing Eyes to end exactly "until your next turn." Canonical catalogs, desktop overrides and relevant legacy/diluted variants agree. All-Seeing Eyes' generic appended reserve explanation is suppressed to preserve that exact ending; mechanics (including free-move reveal persistence) are unchanged. Exact-copy tests cover current projections and authored Dreamwalking.

## Text audit batch three
Text-only removal of Dagger flavor, Appropriate floor clause, and See The Future reserve caveats; tiered Blinking uses the existing range profile. Description construction remains O(L) time/memory for output length L, once per observation/inspection; no simulation, scheduling, RNG, or save-format changes. Focused display tests cover all nine tiers.

## Item grant copy ownership
Item descriptions retain grant names but no longer duplicate ability descriptions. Shared Pack/ground/inspect path remains O(L+G*C) worst-case time and O(L) output memory for L displayed text, G grants and C catalog entries; removes ability forecast/description work for each grant. Existing pickup/drop reconciliation supplies carried jewelry grants to the abilities section; no simulation or save changes.
Shared reserve tutorial paragraph also removed from ability details; specific costs, reserves, and unique rule paragraphs unchanged. No additional runtime work.

### R52 regular Bow normal accuracy and minimal copy

Extending the existing Composite accuracy flag to all bows is O(1) per shot or
preview; it adds no RNG, traversal or state. The exact bow description counts
arrows using the existing O(I) inventory scan and O(1) scratch, where I is carried
item entries. It returns before generic equipment/ability description expansion.


## Revision 53 Commands / Information Help menu and keyboard cursor

See [Help layout, input and rendering](revision53-help-menu.md#complexity) for
fixed authored-text limits, measured wrapping, profile-aware cache invalidation,
scrolling and native/GPU composition costs. The detached Help module cannot
read or mutate a Run. Opening/closing consumes input and invalidates presentation
state; it introduces no simulation command. The detached keyboard cursor uses
O(1) coordinate updates and O(1) state, then reuses existing observation, click
travel, hazard and rendering paths with their existing costs. Plain Z reuses
the existing Wait dispatch. See the linked keyboard-cursor bounds and
verification records; this work is not a published release.


### Integrated-v2 presentation recovery (November 15, 2026)
Restored presentation-only magic field intensity and invisible player ink use constant
arithmetic per emitted cue tile or player cell; existing route traversal and raster
lighting costs remain unchanged. Identity-known inspection checks reuse the core
telepathy query (including its origin and visibility lookup costs); they do not mutate
simulation state or add a second world scan. Venom display scans C condition entries
on each status observation, O(C) time and O(1) extra memory apart from output text.
Replay naming writes B serialized bytes and probes D colliding names: O(B+D)
filesystem operations/work, O(1) additional memory excluding the borrowed serialized
payload. A sibling temporary file plus create-only hard link preserves existing names;
there is no finite collision-count cap (the u64 counter is the representable bound).
Filesystem sync latency and renderer timings are unmeasured. Validate with focused
replay_archive, player_glyph_tests, action_animation_tests, hallucination and inspection
tests, plus full workspace and native screenshot checks. No gameplay balance or
visibility formula is changed by these presentation ports.

### Integrated-v2 mechanics recovery (November 15, 2026)
Current-tree selective port restores species-fixed birth selection and pre-DR attack
coefficients atomically. Birth lookup remains O(R*L) over the fixed catalog and prefix
length; applying a profile constructs one revision51 base actor to restore context-free
stats/energy, then applies the selected profile, with no extra RNG. Saved actors are not
retuned. Ordinary attacks, bow scaling and preview arithmetic add O(1) work and scratch;
Composite scales the rounded post-armor packet once. Normal bow accuracy remains the
latest revision52 distance-independent ratio. Numeric ring effects use the existing
inventory/endpoint scan per query; stacking adds no new traversal. Venom consolidation
scans C conditions once per affected actor upkeep, O(C) time/O(1) scratch plus replacement
key; severity is one packet with the latest expiry. Ability-sight admission reuses existing
terrain/portal route and telepathy queries; actual projectile collision remains separate.
No cache, floor scan, wall-clock timer, RNG draw or simulation revision was introduced.
Same-revision historical replay equivalence is not guaranteed because both source
snapshots use revision52 for different rules. Focused feature and preservation tests
are recorded outside source in the integration evidence; broad QA remains required.

## Riptide two-stage desktop targeting (local integration follow-up)
The desktop retains one optional enemy ID between the enemy click and rally-point
click. No simulation command or payment occurs until the second selection; Escape,
a different ability, or fatal/health-warning cancellation discards the selection.
Each click validates the actor through existing observation/ability-sight helpers:
O(A + V) time, where A is actor lookup and V is the existing visibility/physical
line-of-effect query cost; no new cache or path search is introduced. The retained
selection is O(1). Rally-point admission preserves the core in-bounds-only contract;
ordinary movement blockers still stop each forced pull. Hover preview marks the
candidate square, not a promised movement path. Native pixel capture uses the
existing full renderer (O(screen pixels + observed cells), unchanged). Asymptotic
estimates are not performance measurements.

### Custom ability counters (native HUD)
Each projected ability scans the player's B acquired abilities to identify the first shared-reaction owner: O(B) time, O(1) extra memory per row, O(B²) for all rows, consistent with existing per-row catalog projection. Values use serialized counters/deadlines; no simulation mutation or new timer. Each visible counter paints a fixed-size box/drop and bounded progress meter, O(P + D) native software pixel work (P = indicator pixel area, D = number-glyph raster work/cache misses), O(D) temporary formatting storage. HUD cache keys include indicator values, so counters repaint when only their value changes. Tests compare full/retained/tile rendering at 1280×800, 1920×1080 and 960×1200; timings not measured.

## Twenty-floor blessing rewards (2026-11-16)
The compressed-route flag selects heart blessing rewards and doubling level thresholds in O(1) time and O(1) extra memory per reward/threshold lookup. Heart stat scaling still touches eight fixed fields; no scan or extra persisted item copy is introduced. Sap computes the deficit against the same threshold in O(1). Existing level-up catalog offer selection retains its prior complexity. Native focused regressions cover reward ratios, fractional precision, unchanged stat bonuses, and the first/subsequent cost boundaries; no new performance claim is made.

### Twenty-floor route (2026-11-16)
Route coordinate conversions, adjacent-floor routing, reward multipliers and
level thresholds are O(1) time and memory. Existing floor generation remains the
same algorithm per physical floor, now at most 20 floors for new runs; one-floor
lookahead retained. Save/replay/snapshot flag adds O(1) space. Display conversion
does not affect simulation RNG. Legacy routes keep their existing algorithms.

### Integer player-facing combat quantities (2026-11-17)
Combat forecasts (including Hanged HP/main-energy endpoints and bows), Bestiary stats,
perception labels, and resource-change percentages round only at text formatting.
The shared numeric formatter performs O(1) arithmetic and O(D) output work/storage
for D rendered digits per quantity, at existing UI refresh frequency. Prediction,
resource balances, saves, RNG, and visibility retain their original precision.
Regression checks exercise fractional Hanged values and signed/near-zero labels.

### Measured inventory width (2026-11-17)
Pack configuration reserves the widest Tier 4 Tinkered amulet name from the current catalog. On typography/content/viewport invalidation, it measures E×A labels (E=12 amulet effects, A≤72 divine abilities, excluding Kative) and G carried labels; time O(E×A×L + G×L), with L label length including font measurement, plus existing detail wrapping. Scratch is O(L + G×L); cached scalar advances remain O(distinct glyphs). Unchanged configuration skips this scan. Width is bounded by the viewport; when the measured label cannot fit, it wraps and grows rows rather than discarding text. No simulation, RNG, save, or replay state is changed. Tests cover normal font fit, oversized font fallback, and unchanged-layout caching; no measured performance claim.

### Ability reselection and inspection-only footer (2026-11-17)
- Per mouse/keyboard ability activation, compare the selected ability ID (length `L`): O(L) time and O(1) extra memory; successful reselection clears the two-stage target state without simulation mutation or turn advancement. Existing redraw/preview work is unchanged and not included in this comparison bound.
- Inspector footer reads the existing visibility-filtered report in O(1) time/O(1) extra memory; action/error notices no longer replace inspection content. No new perception, pathfinding, or simulation query.
- Regression checks: `reselect_tests`, `reselect_ability_clears_highlight_and_target_ink_without_a_turn`, `inspect_footer_contains_only_observed_information`, and `cursor_render_keeps_observed_footer_and_does_not_mutate_save`. Bounds above are static analysis, not timing measurements.

### See the Future confirmation and snapshot indicator (2026-11-17)
The desktop confirmation gate examines one command and the owned ability list (A entries, bounded catalog); O(A) time and O(1) additional memory before queuing a command. Its Y/N/Escape reducer is O(1), and cancellation never dispatches or changes save state. Both keyboard and mouse activation use this gate. Snapshot presence is O(1); rendering its box/spiral uses a fixed sample count and constant extra memory. Gameplay payment and temporal capture/restore are unchanged and retain their existing complexity.
Load/restore repair of an owned missing See the Future reserve adds O(P) owned-ability scanning and one expected O(1) map lookup/insertion (constant ID, at most one entry allocated) to reserve normalization. All state validation precedes repair; existing entries and player energy are unchanged. Immutable anchors normalize on the existing restore path.

### Marked-heart presentation (2026-11-17)
Shared item-name formatting adds `(marked)` from existing Borge provenance using constant metadata checks and O(N) output allocation for N displayed name bytes, per item-name observation. No provenance, pickup, altar, heart-consumption, save, or replay mechanics are changed. Pack reuses that name instead of adding a second marker; legacy rows retain their presentation fallback.

### Hostile Borge occupancy (2026-11-17)
Borge body/altar metadata checks are O(1) per placement candidate. Existing movement, path masks, teleport, forced movement/current/fall, revival, and offscreen arrival share the hostile placement guard; population and raw summon candidate lists also check it. No extra RNG draws or persistent gameplay fields. Player interaction and old revisions remain unchanged. Admission/restore repair scans Borge-floor actors and, only for T overlapping living hostiles, scans V cells for a deterministic nearest valid free cell; portal-alias checks contribute P topology work per cell and existing habitat predicates scan A actor trait keys. Worst-case O(F + creatures + T*V*(P+A)) time, O(creatures+T+P) scratch. Plans precede coordinate writes; a floor with no valid destination is rejected rather than silently leaving overlap. Ordinary valid saves incur only the Borge-floor actor scan. Generation installs the Borge event before population, whose shared candidate/admission guards now reserve its cells.

### Ordered animation observations (2026-11-17)
The desktop opts into a scoped, non-serialized core callback during dispatch.
At each action boundary it captures a detached COW Run clone, capped at 128.
After visible receipts are filtered, only the last boundary before each visible
animated action is projected through the normal player-knowledge-filtered
observer; quiet scheduler actions do not each build a View. Raw checkpoints are
then discarded, never passed to the painter or persisted. The callback is absent
in headless simulation. No gameplay RNG, save or replay-command format changes.
Speculative `events::Probe` rollback restores frame references with receipts.

Let F<=128 be checkpoints, C the Run clone cost (including non-COW fields),
R retained nonshared Run data, S<=F selected scenes, O the full observation cost
(visibility rays, terrain, actors and inventory), V View storage, E<=4096 cues.
Dispatch adds O(F*C+S*O+E log E) time and O(F*R+S*V+E) memory. Run clones
share COW floors/actor storage but histories and metadata cannot be treated as
free. Probe cloning costs O(F) references, not deep checkpoint copies. Sequence
lookup is O(S*E); bounds above apply. Active sequencing uses ordinary full-frame
native raster/GPU paint/upload, invalidating the retained tile cache afterward.
Beyond the checkpoint cap, the last safe scene stays until the rest of the cues
finish; no future/hidden scene is substituted. Save during animation writes the
already committed canonical simulation, not a checkpoint.

Performance must be measured on dense/deep rooms and multiple-proc turns.
The rejected prototype observed every scheduler boundary: a 21-creature debug
fixture took ~8.2s versus ~88ms core-only. The selected-scene implementation is
explicitly intended to avoid that regression; do not infer release-platform
performance from debug test figures or from these asymptotic bounds.

Focused validation: the optimized 21-creature debug test measured ~86ms for
observed desktop dispatch versus ~76ms core-only in the same suite run. This
is one fixture, not a release benchmark, and excludes the 17.7s combined raster
capture suite. Four new sequence regressions pass within 34 animation tests;
the transactional-observer rollback/headless regression also passes.

## Persistent divine overlays (November 17, 2026)
Reverse Entropy projects the union of radius-10 portal-aware player endpoint areas on each world observation, including unknown space by explicit design. It stores only perimeter edges, never hidden terrain or actors. With V floor cells, H origins (at most 2) and K portal nodes, endpoint metric setup is O(H K³), then radius projection O(V H K), O(V+K+E) temporary/output storage (E<=4V). Without portals the radius projection is O(V H). Riptide probes at most V distinct cells using shared direction, portal and passability rules; if each probe costs P(A,K), worst case O(V P(A,K)), O(V) storage. It stops at unknown cells and nondeterministic portal branches, rather than revealing blockers or promising a random destination. Neither projection consumes RNG or mutates simulation. Geometry remains detached during animation playback. Painting is O(E + L*C), where L line segments and C cell pixel span; no per-frame pathfinding. Overlay changes invalidate the map in both redraw modes to erase previous lines. Timing not measured; validate with a portal-dense floor and repeated effect toggles.

### Authored divine presentation geometry (November 17)
`animation_shapes` builds receipt-only straight lines and bounded area footprints.
For a line of length L (bounded to the larger current floor dimension), construction
is O(L) time and O(L) extra memory, plus O(L*V) for existing visibility query cost V.
An authored radius R is capped at 12: O(R²) candidates/memory plus O(R²*V) admission.
Called on actual casts/procs, not every render frame. Receipt decoration is O(1).
No RNG, gameplay position/state, save or replay fields change. Runtime renderer cost
and presentation checkpoint memory remain separate from geometry construction.

## Authored divine animation patterns (November 17, 2026)
The desktop renderer consumes only visibility-filtered receipt cells C; no gameplay RNG or simulation time is used. Deterministic nova cellular waves evaluate at most 12 generations of eight-neighbor propagation: O(12 C log C) time and O(C) scratch per cue/frame. Box, line and moving glyph projection are O(C log C) including the admitted-cell set, O(C) space. Event phase scheduling is O(E log E) for E<=4096 cues; each phase and separate travel step finishes before its successor. Input waits for all finite local, projectile, HUD and plane cues, not perpetual ambient motion. Persistent explosion markers are not finite animations. All-Seeing Eyes keeps eleven detached radial snapshots of C view cells: O(11 C log C) setup and O(11 C) additional memory; per-frame view selection is O(11) plus existing bounded action selection. No full runtime benchmark or Windows verification claimed.

### Authored divine presentation routes and flourishes (November 17, 2026)
Death Swift, Erase Metaphysical, Witch Hatred, Mini-Me, Clay Army and See the Future emit bounded area receipts through `animation_shapes` on successful effects only. Radius is 2 for small effects, 6 for Future (helper caps all radii at 12). Each area costs O(r²·V) time and O(r²) scratch, where V is the existing visibility-query cost; no gameplay RNG or saved fields are added. Fortune casts and Sparkle emit O(L·V) observed-cell route receipts (L bounded by floor dimensions); Cogworks uses the existing 7,200-step-capped straight-cell helper before normal visibility filtering. Commandment emits two endpoints per actual displacement step and thus adds O(P·V) receipt work for P already-resolved pushes, bounded by floor tile count. The landed-hit gate checks an already-resolved attack result and replay policy in O(1) time and O(1) extra memory per attack; the policy adds one constant-sized marker per run and constant checks per replay command. All-Seeing Eyes emits an O(T·V) floor reveal receipt with O(T) temporary points at paid activation, not per frame; renderer must remain player-knowledge-aware. These are asymptotic estimates, not measured frame costs.

## Blood Mist (November 17, 2026)
Overflowing emits an inert gas on each spent-blood rescue. A dose is provisionally
100 volume; repeated doses add volume and refresh mixed mist to five environment
updates. Emission checks at most nine cells (O(1)), then existing first-use
environment hydration can cost O(N) time/memory for N floor cells. It preserves
pre-existing gas at the origin and uses adjacent empty/mist cells if necessary.
The normal gas diffusion pass carries remaining lifetime through its contributor
lists: O(N) time and O(N) extra memory per pass on the fixed eight-neighbor grid.
Expiration is O(N) time, O(1) extra space once per world update, independent of
actor haste and presentation time. Optional serialized channel/lifetime arrays
preserve absent fields on older saves. Rewind uses ordinary serialized state;
the experimental four-column gas capture pilot explicitly reports mist duration
unsupported rather than claiming complete coverage. No measured performance
claim. Test with blood_mist focused tests and environment regressions.

## Requested enemy spells and wave presentation (2026-11-17)
- Dryad healing eligibility adds the existing bounded projectile admission query per candidate (A actors; at most 256 branch traces on portal floors), then one committed route for the chosen spell. Scratch and worst-case follow `projectiles::possible_shot_range`; it is not a constant-cost LOS check. Ordinary LOS/range/allegiance remain required. Committed portal forks may miss the selected recipient; they never heal a blocked target.
- Scream presentation scans F floor cells and applies portal distance/LOS predicates where the underlying spell requires them: O(F*(distance+LOS)), O(F) detached admitted cell receipts. No animation RNG or serialized world-time dependencies. Banshee current R52 targeting additionally scans A living endpoints, then existing damage hooks per victim; old revisions retain player-only targeting. Banshee has no artificial new range cap.
- Borge mark telepathy checks F floor records (bounded authored route) per target query; no cache, RNG or changes to generic telepathy immunity. Only the active, unrewarded exact mark grants this sense, in the player's plane.
- Desktop radial wave snapshots are presentation-only; core mutations commit atomically and the renderer exposes each affected location at wave arrival. Tests distinguish core outcome validation from platform/runtime animation validation.

### Guarded Verdania tap (November 17)
At new R52 Verdania population, a single 8-neighbor BFS ranks dry tap sites by shortest path distance from the up stair. V is map cells, C eligible +2-layer roster packs, M<=5 pack size, K unique species. One initial-fuel exclusion scan per K species is cached; planning is O(K*V + V log V + V*C*M) assuming the existing fuel helper is linear, extra O(K*V + C + V). Every legal walking/swimming neighbor of a selected tap must be a safe dry spawn cell and is occupied by exactly one member of one existing pack (no pack multiplication). This immediate approach cut handles alternate paths without per-candidate graph searches. Impossible geometry fails explicitly and uses the existing eight complete-floor retry bound. No per-turn cost, no measured performance claim.

### Desktop continuation pauses (November 17)
Health/continuation input admission is O(1). Intentional fall preflight uses existing
hazard/visibility queries (no RNG); forced-fall presentation retains one O(View)
pre-action observed scene. Outgoing cues are filtered O(E) per paint while held,
E bounded by the receipt cap. Paralysis waiting dispatches at most one normal Wait
per loop/display animation boundary, with O(1) status/choice checks;
never an unbounded synchronous simulation loop. Required pauses or player input stop
waiting. Pauses, queued intent and waiting are ephemeral, not saved or replayed.

## Targeted control amulet travel (2026-11-17)
Per committed R52 control cast, one shared physical ray trace (L traversed cells, A actors, P portal/link-query cost) costs O(ray-plan + L*(A+P)), O(L) detached receipt scratch. It is bounded by the ray engine's existing traversal/cycle limits and consumes no extra visual RNG. Cast snapshots/visibility filtering retain existing event costs. Per aiming preview, reuse of knowledge-filtered Iceblast geometry costs its existing O(observation + floor clone + ray-plan + V*A*P), O(F+V+A+P) scratch; no additional nested aiming recursion occurs beyond one control-to-geometric-item call. Obstruction footprint admission remains existing three-cell checks. No performance timings measured.

## Bestiary fixed-species correction (2026-11-17)
Bestiary R52 reference-depth lookup now uses canonical species keys, including the nest-only Queen, at unchanged O(R*L) fixed-catalog scan cost. Fungus rows call the authoritative O(1) revisioned fungus template factory rather than reconstructing obsolete depth-scaled templates. No extra cache, simulation access, or renderer work. Tests assert current fixed stats and legacy historical ranges independently.

### Death Swift corpse-before-nova presentation (November 17)
Death Swift starts a distinct presentation action after the victim dies. One additional detached observation per proc is O(observer projection) time/memory, subject to the existing 128-checkpoint command cap; headless runs without an observer allocate no snapshot. No simulation turn, randomness, save field or rendering-time gameplay mutation is introduced. Native regression paints the corpse scene before the nova and checks input remains blocked through its duration.

### Irregular nova correction (November17)
Nova receipt envelopes are radial/lobed with a nominal radius plus at most two cells, rather than filled Chebyshev squares. Core generation examines O(r²) candidates and performs ordinary cue visibility filtering; no simulation state or gameplay RNG changes. Desktop recomputes a deterministic weighted eight-neighbor cellular arrival field over C admitted cells: O(C² + C log C) worst-case time and O(C) memory per effect frame (minimum-unsettled selection is linear), C bounded by the radius12 envelope. Small/large novas spread over540/810ms and linger with660/990ms fades, totals1200/1800ms. Timings are artistic calibration, not performance measurements. Test core footprint/immutability and desktop cue/input timelines; review actual-render MP4s for appearance. Visibility holes can truncate propagation; rendering never creates unknown terrain cells.

### Maintained thread presentation restoration (November 17)
Bond and I Came For You project at most two current player-to-target tethers per observed scene. Each costs O(A + L + visible_actor) time, O(L) extra memory, A actors and L <= max(floor width,height)+1 Bresenham cells; visibility uses existing core query cost. Only admitted current terrain cells are retained, and painting does not connect across omitted gaps. Painting costs O(L * max(cell pixel width,height)) alpha blends. Activation uses the same thin geometry with a growing prefix; persistent endpoint changes invalidate the map through existing overlay equality. No simulation, RNG or visibility writes. Motion, off/dead cleanup, timer-sentinel I Came activation, missing observation and gap clipping are regression tested. Alpha and 32ms/cell are visual calibration, not timing measurements.

### Current amulet retirement and unlimited control targeting (2026-11-17)

Retirement is an O(effect-name bytes) read-only predicate per display, targeting
admission and command; it allocates no simulation state. Old saved item homes,
copies and temporal snapshots retain their serialized shape; no migration scan
or replacement RNG draw is introduced. Spawn selection remains one uniform draw
from a fixed-size table (10 current choices, 12 historical choices).

Beckoning removes the old 8–16-cell distance bound, not the map bounds. For floor
width W, height H, ray length L is O(max(W,H)); eight candidate adjacent landing
squares each test L cells. Each step retains existing occupancy/portal/terrain
helpers, O(8*L*(A+P+M+H_t)) worst case, with A same-plane actors, P portal data,
M terrain modifications and H_t habitat/trait queries, O(L+actor clone bytes)
scratch. Preview additionally uses existing observation and visibility cost and
only known cells/actors; no hidden-state leak or extra pathfinding is introduced.
Discord still scans visible actors and traces one map-bounded committed ray, with
unchanged shared ray/portal bounds. Roots has O(1) status work per actor normal
turn after existing terrain lookup, and no damage/proc packet in current rules.

Replay compatibility adds one O(1) command-boundary scalar. Before replay seed
world generation, the recorded policy selects the historical or current roster;
the same policy is propagated through primary floor loot, budget loot and boss
rewards. It consumes no extra random draws. Historical recorded commands retain
their original mechanics until the recorded boundary. Live loading adopts current
rules while retaining existing item payloads. Temporal snapshots preserve the
host policy rather than rewinding the rules amendment.

### Current potion retirement and radial impact geometry
Current potion roster selection is O(1), without rejection draws; the old 20-entry roster remains selected for historical command prefixes, while current generation uses 17 entries. Retirement admission is O(1) and leaves old inventory objects intact. Incineration scans 25 positions (13 admitted cells), invokes the existing fire admission for each, then scans A current-floor actors plus the player for direct burning: O(25*F + A*Q), O(A+13) scratch, where F is existing ignition initialization/topology cost and Q is mutable-target/plane admission cost. It introduces no frame-driven simulation RNG. Shattering scans (2R+1)^2 positions, R=6 current / R=2 historical, with existing topology and structure exclusions; shared obstruction teardown remains proportional to affected structure cells. Darkness retains its radius-3 mask, duration and illumination calculation. Presentation waves are separate from authoritative simulation; rendering does not advance turns. These are code-derived bounds, not measured timings.

## Radial potion presentation (2026-11-17)

New potion receipts enumerate C admitted cells from a W×H floor in O(W H) time and O(C) detached memory per applicable potion. Telepathy carries uniform floor geometry only; identities come from the normal player-filtered observation. No wall-clock/RNG state enters the simulation. The host observes the exact completed action once and builds 11 presentation frames, revealing cells and creature identities only at the same quantized ring arrivals used by the tile-light renderer; Shattering waits 120 ms after the blue flash. Each frame clones a bounded View (V cells, A creatures and other UI), filters O(V log C), and performs current actor reconciliation O(A V + A²). Extra memory O(11 × View size + C), limited to the existing 128 action snapshots/4096 receipts per command. Map layers at a coordinate are preserved. Terrain is held at its captured appearance during the 900 ms wave, rather than reconstructed from a future observation. Paint uses the precomputed frame and enumerates C receipt cells for its light ring; no scene cloning per paint. Estimates are unprofiled; test full-floor telepathy with maximum actor count and inspect capture/paint timings before optimization.

## Approved player status screen tints (2026-11-17)

The presentation-only screen veil scans S detached player status labels and at most C detached cells to find player illumination: O(S+C) time, O(1) scratch per paint/invalidation comparison. Five approved color families plus existing immersion are blended by an order-independent mean with fixed 34/255 opacity; aliases do not multiply weight. Tint transitions invalidate the complete native frame in both retained renderers. Painting remains O(P) for P damaged native pixels (one GPU alpha quad); no turn, RNG, visibility, or serialized simulation state changes. Unchanged retained pixels are not tinted again. Bounds are code-derived, not benchmarked.

### Current ring roster and retirement (2026-11-17)
Current versus historical roster selection and removed-power admission are fixed O(1) checks with no extra RNG draws or rejection loop. Ordinary loot selects uniformly from a ten-entry current or fourteen-entry historical roster. Each existing owner-aware named-power query still scans I carried inventory items and C ownership conditions: O(I+C) time and O(1) scratch. Retirement display is a fixed lookup plus O(N) output for N name bytes; saved item objects and separate grants are not modified. Island Swimming placement retains its prior connectivity/current-reachability bounds. These are code-derived complexity bounds, not measured timings.
The independent ring policy marker is an O(1) command-boundary check. Temporal snapshots and future restores preserve the live host marker rather than capturing it as rewindable gameplay; old journal tails execute the historical policy and only at EOF adopt current rules and invalidate old append authority. No extra RNG draws are introduced. Genuine pre-change seeded replay fingerprints and provenance are retained in `fixtures/ring-prechange-r52.*`, produced by `tools/generate-ring-baseline.rs`.

### Willpower finite-status admission (2026-11-17)
Each newly applied finite duration performs one owner-aware ring scan, O(I+C) time and O(1) extra memory for I carried items and C actor ownership conditions; scaling is performed before merging with any existing countdown. Potion buffs, curses, ordinary status packets, recurring gas contacts and amulet statuses use the target's equipment, not the caster's. No frame, save/load or countdown-decrement pass rescales timers; indefinite traits, terrain lifetimes, resource counters and cooldowns remain untouched. Historical ring-policy command prefixes retain their existing duration admission. Values are code-derived bounds, not profiling measurements; validate repeated contacts and current/legacy potion/status regression tests.

### End-of-run Space acknowledgement (2026-11-17)
The terminal-scene keyboard check adds one fixed key comparison per keyboard event: O(1) time and O(1) scratch. It opens the existing summary without dispatching a Wait or changing simulation/replay state. Gameplay acknowledgement inventory is recorded in `pause-audit.md`; existing animation locks and explicit yes/no choices are unchanged. Bounds are code-derived, not measured.

### Five-turn status-gas refresh (2026-11-17)

For each existing environmental exposure, select the policy and one authored duration in O(1), then use the existing recipient ring lookup (O(I + C), inventory items plus copied/equipment grants) and max with the countdown in O(1). Extra memory is O(1) per actor; no new grid pass, RNG consumption or stored gas timers. Across A exposed actors the existing duration queries cost O(A(I+C)); gas propagation and normal status expiration costs are unchanged. Bounds are existing inventory/actor limits. These are code-path estimates, not new profiling measurements; validate with current_gas_status and replay-boundary fixtures.

### Samara maximum-HP rule confirmation (2026-11-17)
No runtime algorithm changed: each existing razorleaf contact computes 2% of maximum HP in O(1) time and O(1) scratch before the existing damage pipeline. Existing emitter-field generation and actor exposure iteration are unchanged. Regression fixtures check wounded actors and serialization; they are not performance measurements.

### Cosmetic creature glyph refresh and Bombardier flicker
- Inputs: detached creature identity/profile and public tile coordinates; no simulation mutations or RNG.
- Identity defaults resolve once per admitted creature observation in O(1) time/space. Frozen catalogs and serialized actors retain their replay-compatible identities.
- Live directly visible Bombardier glyphs vary orange brightness using two smooth cosmetic waves. Paint O(1) per glyph; one boolean per cell. Existing retained/tile dirty planners enroll those cells for ambient repaint, not the full map. Corpses, telepathic sensing, explicit appearance overrides and reduced-motion snapshots do not flicker.

### Floor-entry See the Future (2026-11-18)
Let S be serializable world nodes/metadata, F floor blocks and D visited floors (bounded by the dungeon). At each real floor entry or first live admission, one immutable FutureAnchor shares COW floor/actor data, clones metadata, and performs an O(S) finite-state scan. One anchor is retained, not one full world per floor; used-floor tracking is O(D) time/space. Ordinary availability checks are O(D), with D <= 40 in legacy dungeons (20 displayed in the current route). Restore is O(S) clone/validation plus O(D) ledger transfer; first save encoding of an anchor is O(S) and cached, subsequent save output still O(S). Core turn cloning shares the anchor in O(1); nested anchors are stripped during capture. Native tracker rendering visits exactly twenty numbers. No runtime measurements are claimed; exercise new `future_floor` unit/integration tests and the native tracker screenshot regression.
The HUD converts the spent-floor list to a fixed u32 mask in O(D+20), with O(1) row storage. Native paint is bounded to twenty glyphs/X marks and one state dot; existing font cache miss/rasterization costs remain. The retained HUD signature includes both indicator and active state so a rewind invalidates this row.

### Quiet doors and nonblocking corpse feedback (November 2026)

Superseded visually by the later plain-highlight removal below: corpse collapse
receipts are now dropped before scheduling instead of painted without a lock.
Door-open receipt admission is an O(1) label check per actor event; mechanics,
visibility and logging are unchanged. Ordinary non-player-owner death collapse
receipts are tagged after admission (owner lookup retains its existing cost).
The desktop still paints their 100ms feedback but excludes their duration from
phase scheduling, held-line lifetimes, scene checkpoints and input/replay gates.
For E receipts (cap 4096), scheduling remains O(E log E) time/O(E) scratch;
blocking checks are O(E), and the scene sequence adds O(E) filtered references.
Existing scene observation and raster costs are unchanged; no simulation clock,
RNG, or gameplay state depends on this classification. Timings are authored,
not measured frame-rate guarantees. Focused receipt/scheduler tests cover quiet
doors alongside other effects, player-vs-enemy death, motion modes, and mixed
ability/death action windows.

### Plain tile-highlight removal
See [quiet-tile-highlights.md](quiet-tile-highlights.md): O(E) allocation-free
presentation admission before the existing O(E log E) scheduler, with unchanged
specialized rendering costs and no simulation-state or RNG changes.
## Screamer one-shot lockout (2026-11-19)

`S` = pending successful screamers, `D` = deadlines due this world tick, `A` =
existing scream-recipient count. Scheduler space O(S), at most one keyed entry
per actor. Scream eligibility uses an O(1) per-actor owner/deadline receipt; insertion/replacement
is O(log S). Duration aggregation adds O(1) time per recipient and O(1) space to
the existing recipient pass; it adds no new scan and does not change that pass's
hostility/portal-distance/visibility helper costs. Global tick expiration performs
one queue peek, then O(D log S) queue work and D indexed actor resolutions, never
an actor/status sweep. Actor resolution has existing radix-index depth cost;
structurally dirty registries may rebuild in O(A_total), and actor-page mutation
retains existing copy-on-write costs. Dead actors' events survive only until
their original deadline. Queue serialization, clone and snapshot materialization
are O(S) time/space; policy admission and replay-boundary checks add O(1).
These are source-derived bounds, not measured benchmark claims. Focused tests
exercise actual five-phase status expiry, both variants, gates, identity,
serialization/rewind/offscreen removal, legacy policy and journal migration.

Offscreen proxy ticks expire their due queue and return it with actor mutations.
A constant-size per-caster historical receipt survives global tag cleanup, so
catch-up at an older tick still gates correctly without scanning or transferring
queue scope when an actor changes floors/planes. Proxy Run clones include O(S)
queue clone cost in addition to their pre-existing costs.

Current R52 admission checks the Avatar's plane and existing portal-aware
distance once per ready Screamer before payment, cooldown, or recipient
enumeration. This adds one distance query and no new traversal or storage
beyond the command-index compatibility marker.

## Uniform complete Avatar starters (2026-11-19)
See [avatar-starters.md](avatar-starters.md): one O(A + 6^3*(D+E)) tuple
construction, O(N) bounded rejection draws with N<=216, and O(A+N) reference
scratch. Isolated RNG preserves historical global starter draw advancement.
## Mind Read persistent outline (2026-11-19)

V = floor cells, A = actors on the current floor, B = player ability/toggle
entries, H <= 2 sensing origins, E <= 4V boundary edges. Per view observation:
ability/toggle admission and core vision origin lookup O(B+A),
Euclidean membership O(VH), perimeter extraction O(V), O(V+E) scratch/output.
No portal graph, hidden terrain lookup, ray tracing, actor visibility queries or
simulation mutation is added. Persistent painting costs O(E) logical rectangles
plus the renderer's existing clipped raster coverage; detached overlay equality
participates in existing full-map invalidation on movement/toggle changes.
The same perimeter/stroke helpers retain Reverse Entropy behavior. Bounds are
source-derived, not timing measurements; deterministic tests compare every cell
against core telepathy for both toggle states and Half Here, hidden topology
invariance, and pixel masks/colors matching the entropy stroke.

**Spent-ember contact.** Each actor arrival, environment exposure, or creature
hazard probe checks the existing cell kind and rule marker in O(1) time and
O(1) additional memory, with no added passes. Over A actor arrivals/exposures
and H hazard probes this adds O(A + H) comparisons. The save/replay boundary
is one optional integer per run/record and is checked in O(1) time per replay
command. No additional randomness; rejected ember contacts draw no RNG.

## Stair versus fall acknowledgment (2026-11-19)

Three command-local Boolean travel-cause flags add O(1) time per downward travel
and O(1) storage to the existing presentation Batch. Only an installed observer
records them; speculative probes already clone/restore that Batch. Desktop
classification adds O(1) at dispatch completion, with no actor/floor scan, extra
world snapshot, rendering pass, RNG call or serialized state. Existing outcome
snapshot cost is avoided for stairs-only descent and retained for actual falls
and other previously acknowledged downward transitions. Tests cover direct and
Move-triggered stairs, forced flight-expiry fall onto water, deliberate chasm
preflight/one acknowledgment, speculative rollback and mixed stair/fall receipts.
## Passive Ragelight
The toggle admission and policy checks take O(1) time and memory per command.
A lethal event retains the existing ordered acquisition scan and reserve
payment. Passive Ragelight adds no per-turn work; it skips its active-list
lookup and failed-rescue toggle removal in current runs. Replay/saved-session
admission tracks one O(1)-sized command boundary; only older journal tails are
replayed under legacy toggle rules. HUD rendering removes a fixed-size orb
without removing the reserve bar or cost details. The adjacent red next-cost
box uses the same live counter, with O(D) text layout for D displayed digits.

**Steam exposure.** Reuses the existing authoritative one-gas-per-cell
world exposure and visible hazard projection. Adds one O(1) gas/rules check
per eligible actor exposure or hazard probe; successful packets use the same
existing damage, immunity, and inventory resistance lookup as Burning.
No new floor scan, gas layer, timer, RNG draw, or per-actor state. The replay
switch adds one optional integer per run/record with O(1) lookup per command.
## Execute threshold crossing
The landed-weapon path uses the ordinary resolved HP packet exactly once and
checks one health-owner lookup and three scalar bounds after mitigation and
redirection. No extra rolls, hit packets, or recovery scans. Successful
finishes use the existing reserve-first payment and terminal death path.
Replay adds one constant-size command index, with legacy journal tails
replayed before the new rule is admitted.
## Beckoning action receipt (2026-11-19)

Per successful pull, O(d+B+C) worst-case indexed source/target resolution,
existing owner/hostility checks and condition-map growth, where d is the
creature-index radix depth, B the relevant ability checks and C target conditions.
Each scheduled action adds O(d) receipt lookup when absent; consuming a receipt
costs O(d+C) because IndexMap shift-removal preserves order. Existing actor-page
COW costs apply on writes.
No new actor scan, timer scan, queue or periodic decrement is introduced. Storage
is one scalar owner stamp per marked actor, O(S) for S marked actors (including
copied stamps that cannot suppress a different identity). Existing
snapshots/serialization include these conditions; policy admission/replay checks
add O(1). Dirty registry rebuilding retains its pre-existing O(A*d+F) costs.

**Ichor flame spread cadence.** The existing scan adds one O(1) fire-kind
and world-tick gate per established source, O(F) extra work over F sources
and O(1) extra scratch, with no new loop or independent RNG draw. On held
steps the usual at most four cardinal exposures are omitted; same-cell and
direct exposure paths still run normally. Active and exact inactive-floor
steps share the authoritative tick. The replay boundary is one optional
integer, not per-floor or per-cell timing state.

### Entropy / ichor immunity distinction
The existing damage-packet admission guard gains a constant-time kind/revision comparison. Per packet, the unchanged bounded number of ability membership scans costs O(A) for A actor abilities, with O(1) additional scratch. No actor, tile or status scans, serialized state, scheduling or RNG were added. Ordinary liquid exposure remains unchanged.

### Grouped Commandment presentation
For V actual push steps and C independently admitted route cells, collection adds O(V) existing visibility admission calls plus O(C) amortized vector work and O(C) detached presentation memory. It emits one event rather than one per step, deduplicating only consecutive shared endpoints. Rendering is O(C log C) from the existing shape-cell set construction plus O(C) tile iteration per frame; duration is bounded at 240ms independent of route length. No interpolation, RNG, serialized state or simulation scheduling changes. Slippery removes one presentation call only.

## Outcome-only short god novas
Each committed effect emits one bounded radius-two cue, O(C) candidate cells
plus existing plane/actor/tile visibility filters, at most 81 iterations.
Dream entry uses the normal destination-plane observation clone, sharing COW
floor/actor blocks and copying O(F + M) section references/metadata.
Visible receipts carry exact authored color; desktop geometry uses existing
O(C² + 8C log C) cellular arrival and O(C) scratch, with O(C log C) radial
fallback for disconnected admitted cells. These receipts have no input-hold
duration and use no simulation RNG, clocks, or serialized fields.

### Live R52 divine balance

A replay-policy marker adds O(1) serialized space and O(1) checks at startup,
journal/replay admission and command boundaries. Overflowing adds one bounded
stock condition per actor after a save is spent. Acquisition defaults to five
without a write. Each current damage packet checks the resolved owner's HP and
fixed key (expected O(A) actor lookup and O(1) condition lookup; only an eligible
full actor scans its K owned skills and performs the existing COW-owned write).
The player's command-end check and level-up use at most O(K) on that single
actor; no floor scan, RNG, or clock work. The old per-50-damage bank loop remains
only for historical rules. Lifesteal rate, sleep duration and dream price use
fixed arithmetic and no extra stored state. Low-energy Sleep Walk adds an
existing effective-stat lookup for a capped owner away from zero (cost depends
on equipment and conditions); AI, movement and observed animation checks share
the threshold. The animation snapshot adds one runtime boolean per observed
actor, not persisted state.

## R52 trailing Hellfire

The second staircase-linked frontier advances every other collapse pulse.
Per fire pulse, expansion is O(K) for K cells in its bounded-degree frontier,
plus the existing floor lookups and copy-on-write overhead. Each cell joins
each frontier at most once. The persisted per-floor mask is O(V) for V cells
when the fire arrives; the frontier and changed-cell scratch are O(V)
worst-case. Flame contact, ordinary secondary fire, and rendering reuse the
existing Hellfire terrain path. There is no new per-frame simulation or RNG
draw for frontier growth.

R52 Blood Mist fire exclusion is O(1) at each existing fuel-layer query, with
no additional allocation, RNG draw, or map scan. Its save/replay provenance
is one optional command offset, checked in O(1) at admission or playback.

### Portal-routed sight animations
Sight-based visual links reuse the optical or physical line-of-effect portal
route; committed projectile routes are unchanged. With K portal endpoints and
maximum segment raster length R, route lookup is O(F + K²R) time and O(K + R)
scratch (F is floor lookup); flattening S route cells adds O(S) time and space.
Only admitted observed cells paint, and the renderer does not fill a portal's
screen-space gap. Persistent threads do the same query on their existing
observation pass; no new simulation, randomness, or per-frame route search.

### Chronicle and hover copy
The Chronicle visibility predicate scans at most O(L) bytes in a log line of
length L with O(1) scratch. It runs during the existing recent-window and
full-history passes, preserving raw archive indices for bestiary notices.
Directional copy adds O(1) work per completed current-rules floor transition,
before the existing log append. Hover no longer adds a biome-description line.
Wendigo titles use the stored name; no change to actor growth or rendering state.

## R52 Dreamwalking footsteps and Death Gas

A stomp attempt (arrival or Wait) now checks projection state in constant
time and skips the existing O(A) floor-actor scan when dreamwalking. Awake
stomping and corpse revival do not acquire an extra scan.

Death Gas adds one optional channel to the existing single-gas-per-cell
propagation grid. For N floor cells, an existing initialization or view refresh
can spend O(N) extra time checking for this gas or filling its channel; a
used channel adds O(N) volume/provenance storage. For D deposits that invoke
initialization in a turn, the worst-case additional scan cost is O(D × N).
Diffusion still uses the same bounded neighborhood and RNG order. Contact adds fixed gas-type tests
and bounded ability-list checks to the existing damage/status processing on
exposed actors. UI presentation reuses the existing gas blend per observed
cell and hazard-inspection paths. Replay admission and temporal preservation
add one scalar command-boundary stamp; no new world copy or event log is
required. These are analytical bounds, not benchmark results.

## R52 enemy breath animation

See [enemy breath presentation](revision52-enemy-breath-animation.md) for
the exact footprint, visibility and action checkpoints, and the bounded
travel-band and compositor costs. No environment or damage rule is changed.

## R52 Minotaur attack admission

The shared strike gate reads the optional replay boundary and scans the
attacker's existing trait list, O(B) for B traits and O(1) extra memory.
A committed charge adds one traversal-admission check before its existing
actor scan (O(A) contact checks with the existing topology lookup for A
floor actors). Those helpers reuse the existing terrain, topology, and
protection queries. No second path field, turn, or random draw is added.

## Forced chasm landings (native)

A forced fall reserves landings in creature-ID order; equal distances still favor
row-major order. With C candidates, V destination cells, As source actors,
Ad destination actors, and F generated floors, candidate collection costs
O(C*As + C log C) and occupancy initialization O(Ad). An unobstructed exactly
aligned destination takes one eligibility query. Otherwise the scan is
O(V + Q*E) per falling creature, where at most Q <= V queries can improve the
current best distance, each with cost E. Eligibility still includes portal
alias occupancy, barred terrain and structures, and active bulwarks. Portal
and linked-area lookup visits up to the saved link/pocket counts and any group
on the queried cell; temporary stair and altar checks retain their own
modification/object scans. Reservations and cohort snapshots take O(C + Ad)
scratch. The active source and landing eligibility no longer need whole-run
clones; a different source depth still needs a playerless rule view, and
final impact still uses its normal transactional view.

Generated revision-52 destinations skip the generation transaction clone
while retaining allocator validation (clean constant-time after the O(F)
floor lookup, full index reconciliation if dirty). An ungenerated destination
still synchronously pays generation; an involved destination still pays exact
catch-up and starts receiving offscreen turns once activated. Impact,
visibility, floor copy-on-write, and catch-up costs are additional to the
placement scan.

### Borge altar projectile offering (R52)
- After an ordinary committed Throw traces the ray and reports its landing,
  the new check is O(1) time and scratch per throw: one current-floor event,
  exact landing coordinates, and actor/depth provenance. No extra ray,
  visibility query, random draw, or environmental tick.
- On acceptance, taking one heart is O(I) worst-case for inventory Vec
  removal (I carried entries). Appending the shard is O(G) worst case
  if the depth's G ground items must copy on write or reallocate; amortized
  O(1) on a unique vector. The same O(F) current-floor lookup bound as
  other floor accessors applies where they search F stored floors. Stepping
  onto the satisfied altar uses the unchanged ground pickup pipeline.
