> Appearance override: the latest designer instruction replaces the historical decorative silhouettes and scene crossfades below with tile illumination or moving canonical glyphs. Trigger/geometry/privacy coverage remains applicable; old style-specific raster receipts do not certify the new look. See [Brogue-style contract](revision52-brogue-animation-style.md).

Current presentation is defined by [Brogue-style primitives](revision52-brogue-animation-style.md) and the later [magic/projectile travel amendment](revision52-magic-projectile-travel.md). Historical vector, crossfade and fixed-140-ms descriptions below are superseded by those entries.

# Approved R51 proposal implemented for new revision 52

The source proposal has exactly 170 labels (P01–P74, A01–A12, E01–E84).
The full keyed implementation/test matrix is [animation-r52-coverage.csv](animation-r52-coverage.csv).
Player, enemy and inventory detail matrices link actual trigger locations rather than treating taxonomy coverage as gameplay verification.

## Boundary and geometry

`PresentationEvent.animation` is optional command output, not part of Run, canonical saves, snapshots or replay inputs. All new emitters return before recording for revisions below52. A TLS batch belongs to one engine command transaction; failed/probed commands discard it. UI timing never consumes simulation RNG, advances a turn or writes an actor.

New cues contain exact already-resolved cells. The compositor never interpolates a world line, fills an implied radius, or connects portal exits. Projectile physics records its committed traversal and carries that detached route through attack resolution; pure preview queries never record routes or perform a random fork. Amulets carry their admitted trace/area, including empty casts, independent flood lanes, and actual affected wall cells.

An event is admitted at its actual emission time: actor cues require the player's current plane and direct visibility/all-seeing, identity-bearing enemy effects are suppressed during hallucination, and terrain cells require direct observation. An inactive floor's proxy never emits. Final painting additionally masks the current known view and applies its actual illumination, including Darkness overrides. This conservative policy does not give telepathy new terrain or identity information.

## Appearance and timing

Tile-local strokes distinguish cuts, droplets/venom strengths, clasps/roots, chevrons, eyes, shields/fractures, bolts, slabs, glyph folds, rings, membranes, waves, sparks and inventory bottle halves. Gods retain their authored palettes; time-weave remains teal. Same-family shared triggers are intentional, not a catch-all screen flash. Existing static statuses, terrain, lighting and creature trait text carry persistent state. Static-only proposal entries have no added recurring loop.

Per action, travel starts at0ms, hit/miss at90ms, special at125ms, damage/heal at170ms, and death/rescue at200ms. Every action finishes within300ms. Separate actions stagger by50ms for at most150ms, then overlap; even a crowded room finishes its batch within450ms. This bounds presentation latency rather than serializing dozens of creatures behind the already-resolved board. Gameplay/input never waits. A new normal tick, floor or rewind clears previous cues. Same-tick free actions overlay without extending a delayed queue.

Normal, Reduced and Instant action modes are persisted in application display preferences, never game rules. Reduced uses static marks. Instant removes stage delay and reduces delivery to observed endpoints (retaining true wave/cone areas); it does not show a travel line. The global reduced-motion preference also applies. No camera shake or flashing screen veil is introduced.

Inventory feedback uses actual item IDs and player-owned resources. An open Pack marks the matching visible row; a consumed or scrolled-off item falls back to the Pack heading/button. Energy-only marks use the Energy HUD even with Pack open; health, ability-charge/unlock, and acquisition-capacity cues use their corresponding HUD/Pack anchors. Nothing is placed on the ground to represent an inventory reward.

## Complexity

Let E be recorded cues (cap4096 in52; legacy128), C their total requested cells, A observed actors, K actor metadata, Q a visibility query including cold portal/lighting guards, V floor cells, and S stroke/pixel cost. For N candidate emissions, event filtering is O(C*Q) time and O(C+E) detached memory. Once the queue cap is reached, each accepted overflow uses Vec.remove(0), shifting O(Ecap) entries (Ecap=4096 in52), so worst-case prefix eviction adds O(N*Ecap), not O(N) append cost; multi-cell cues do not reconstruct physics. The committed ray recorder copies the physics route once, O(route length). Probe copies include the bounded queue and route metadata.

Player command snapshots cost O(A*(K+Q)) capture and O(A*(A+K+Q)) comparison with current actor lookup. The E53 lighting transition takes two full terrain observations only on a floor containing a darkness source, plus O(V) comparison/memory; it is not O(C) merely because only changed cells paint. E67 stores already observed disguise identifiers only. Renderer scheduling sorts O(E log E), each frame projects O(E+C) cells then existing viewport filtering. Raster strokes use at most150 bounded local segments (25 actual removed-status symbols × 6, otherwise at most40) per mark, O(S*max(cell width,height)) native pixels, including the GPU/dirty-overlay paths. No performance measurement is implied by these bounds.

Plane/floor transition capture uses only the outgoing detached observed View, never a second-plane world query. The map layer cross-fades for250ms with GPU-supported alpha quads/glyph masks; UI sheets are never captured. Reduced/Instant skip it, and a new command discards it. Retained repaint marks the map rectangle until expiration. This adds O(C) detached View memory and O(C + painted pixels) work on transition frames only. Static Fated Duel edges are directly observed, anchored to the original5x5 area, and participate in normal retained-cell invalidation.

Inventory Tinker uses three gear strokes before the actual granted-god spark; symbolic transfers retain only two admitted endpoints, with directional family-specific marks and no invented intermediate route.

## Mechanics and verification

No combat, immunity, costs, displacement admission, terrain propagation or RNG rules were changed to match proposal prose. The proposal's open Iceblast blocker and Sparkle-origin ichor-water questions remain design questions; current executable rules are retained.

Core focused receipts: verification/r52-animations/core-focused-5.log (48 passed), core-focused-6.log (55 passed), core-focused-7.log (62 passed). Each records its actual source checkpoint; later fixes need their own subsequent receipt. The full170-row CSV lists concrete tests per applicable entry and explicitly labels source-only rows. A shared test or tag is never counted as full gameplay verification for every entry.

Native test capture sets are under verification/r52-animations/native. The initial actual six-amulet and two potion-remainder captures passed, while the original silhouette test exposed an invalid100px whole-canvas fixture (corrected). Later native build retries uncovered test-only type/import errors and are recorded separately. Later receipts supersede those failed checkpoints: core13 passes73/73 selected tests; native7 passes23/23 selected tests after type-only preflight. The final P61 construction/removal core and native scenarios subsequently passed one selected test each; eight actual frames were inspected. Their receipts are in verification/r52-final-construction.


## Final bounded refinement batch

Paid player command cues receive a distinct action ID after the haste phase; skipped IDs are permitted. Player snapshot deltas reuse that saved command ID, while free commands remain action0. A later enemy or status group cannot inherit the initiating command's ID. The core12 receipt passes69 focused tests, including a real free toggle and a separately paid command with reacting enemies.

Native6 passes22 selected tests after type-only preflight. It includes actual A01–A12 casts, retained and consumed potion rows, open/closed responsive HUD anchors, actual Dreamwalk and Wake, legacy no-crossfade, persistent Duel anchor, oriented families, and tiled/full fade parity. These are selected representative scenarios, not170 individually verified playthroughs. The keyed matrix distinguishes source-inspected entries and named executed scenarios.

The subsequent final refinement adds exact wall-hit chips, observed charge obstruction, positive-to-zero status fades, source-view Ascend columns and a known outgoing player-glyph outline. Status expiry snapshots add O(A*(K+Q)) time and O(A*K) temporary memory at early/late/environment boundaries for A actors, K actor metadata and Q observation admission, with bounded status-key comparisons. The continuous-Yeti exception adds the existing Half Here origin, inventory-ring and portal-distance helper cost for an ending Slow; that helper may scan actors/inventory and is not assumed O(1). Fire Breath's detached reached-cell ordering adds O(C*portal-metric + C log C) time and O(C) memory; the actual ignition/RNG order is untouched.

The desktop dispatch wrapper conditionally builds a full outgoing observation for every R52 command while Dreamwalking, as well as commands capable of entering or changing a plane/floor. This costs the existing observation algorithm plus O(View bytes) detached retention even if no transition occurs, and O(I) dagger lookup for I inventory items. Reduced/Instant skip painting but this implementation still takes that conditional snapshot. Transition painting adds only a bounded source-player outline/column to the known View, never queries a hidden body or the destination plane. Failed Teleportation uses a small local fold-and-reopen without relocation as its visible failure signal.


Selected evidence: core13 and native7 cover the shared animation systems. Native7 includes the requested quiet-UI shared-resource label removal. Two direct native label regressions pass separately. The final P61 pit-sink/pit-rise/slab-sink change also has passing selected core/native tests and eight inspected frames. The unfiltered workspace and Windows build gates remain separate requirements. Existing captures are genuine native raster output; a contact sheet is only a labeled derivative. No screenshot or focused count is presented as all170 scenarios tested.
