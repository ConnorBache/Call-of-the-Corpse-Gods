# Native Flesh palette and cosmetic blood, October 25, 2026

## Scope and boundaries

The native gameplay presentation now uses warm red wall/floor/substrate colors,
crimson rivers and shallows, and restrained deterministic vein-like color ridges
under the existing ASCII glyphs. No raster creature art or geometric decals were
added to gameplay. Vegetation retains its separate green identity. Ichor retains
its green-gold identity, now strongly separated from the crimson river palette.
The visible existing blood-stain layer gains a restrained red ground tint.

Rivers remain the exact `water` / `shallow-water` simulation tiles. Movement,
currents, drowning, swimming, habitat, fire interactions, visibility, illumination,
feeding, healing, and resource costs are unchanged. No blood-specific darkness
or other new rule exists. This is a material-palette change, not a new terrain
layer. User-edited material colors and glyphs remain authoritative; default
water illumination anchors receive matching crimson equivalents.

All visual changes consume the existing knowledge-filtered terrain observation.
Unknown cells remain absent. Remembered blood comes only from existing hazard
memory. Cosmetic spatial noise derives from seed and coordinates, never gameplay
RNG or clock-driven simulation. Static frames remain repeatable.

Revision31 Hemogoblin, Hemogoblin Bloodmancer and Hemogoblin Inventor movement
marks the arrived-at ordinary floor cell in the existing serialized `blood`
boolean layer. These footprints are cosmetic. The Mech, dead actors and non-floor
tiles do not mark it. The shared movement hook covers normal/haste, forced and
offscreen movement consistently. It does not set `dirty`, add events, consume RNG,
or alter actor conditions. Existing save, hazard-memory and temporal snapshots
carry the same layer without a new schema or history buffer. Revision29/30 keeps
its original footprint behavior for replay compatibility. This does not add
feeding, blood harvesting, tracking AI, or player movement effects.

The persistent floor-one entrance is projected as a bright blue Omega on dark
blue only when its underlying up-stair terrain is known. Its semantic tile stays
`up`; the dedicated entrance implementation owns geometry and gameplay rules.

## Historical parity and native tests

The original v48 appearance profile, renderer-color helpers and golden fixtures
remain intact. The observation adapter has an explicit frozen palette route for
historical RGB differential tests. Live native observation and cosmetic animation
frames use the new Flesh palette. Behavioral and native-presentation regressions
exercise the live path independently; the source RGB tests are not a claim that
the updated native aesthetic looks identical to v48.

Focused tests cover material overrides, unchanged ASCII semantic identity,
deterministic ridge colors, crimson rivers versus Ichor, blood visibility,
unknown-space redaction, entrance identity, cosmetic footprint-only mutations,
legacy revisions, save round-trip and repeated movement. Set
`CORPSE_FLESH_REVIEW` to a directory and run the desktop export review tests to
produce actual native-rendered PNGs. The generated-floor screenshot is gameplay;
the side-by-side material sheet is an explicit detached synthetic visual fixture.
Windows input/fullscreen verification is separate from these renderer tests.

## Complexity

Let V be floor cells (validated maximum 3600), C <= V observed cells, M be a
material object's serialized size (fixed current catalog shape), and A the
native drawable pixel area. Palette substitution does three fixed field
comparisons/updates per material, O(M) including the existing detached JSON
material clone and lookup costs, not a claim of allocation-free O(1) processing.
The bounded neighbor blend examines at most four already-redacted cells, so
material preprocessing is O(C*M) time with O(M) transient scratch per cell.
Two interpolated value-noise samples per ordinary ground cell add O(C) arithmetic
and O(1) scratch. No new persistent map cache is retained. Current terrain and
animation frames retain their prior visibility/light-field and O(C) cell/buffer
costs; rasterization and upload still require O(A) work. Color changes do not
increase the number of glyphs, textures or draw primitives. Existing font-raster
cache misses retain their documented costs.

Blood footprint stamping is O(K) definition lookup, K the bounded authored
creature definitions, plus O(1) cell mutation per movement hook. The first stamp
on a floor allocates its existing V-bit logical boolean layer in O(V) time and
O(V) bytes; later stamps allocate nothing. Arc copy-on-write may clone the floor
when it is shared with temporal history: that existing clone is O(F) time/memory
for the floor's full retained data F and is not omitted from the movement cost.
No dirty scheduling, pathfinding, LOS or random draw is added. Serialization and
temporal capture include the optional existing blood layer: O(V) additional
serialized values when that layer was previously absent. These are asymptotic
bounds, not wall-clock timings; profile generated maps and long temporal history
before making speed claims.

The already bundled, OFL-licensed Liberation Serif face is now the final map-font
fallback because the existing subset faces lacked uppercase Omega. Existing
font indices, glyph choices and ASCII atlas remain unchanged. Startup decodes
one additional existing font, O(B) time/memory for B font bytes plus the fontdue
parsed representation; cold Omega rasterization uses the existing compound
font path. Its rendered mask is verified not to be the missing-character box.
