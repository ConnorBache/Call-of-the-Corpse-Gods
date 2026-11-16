# Plain player glyph (November 2026)

Designer instruction: “Make the player glyph just an @ with no super script.”

## Scope and boundary

The immutable desktop actor-to-cell projection now emits `@` and an empty
superscript for the actual player in every game mode and admitted simulation
revision. This intentionally updates presentation of old saves and replays
without changing their simulation revision, saved glyph, species, stats,
abilities, RNG, commands, or replay policy.

The Dreamwalking spirit is the actual player and uses the same projection. Its
separate tagged sleeping-body endpoint also displays `@`, including the creature
list's copy of that glyph. Its sleeping label, shared resources, normal-plane
blue ink and inspection details remain intact. There is no separate soul entity
or soul-render path in the native core; dream/player forms pass through this
same actor-to-cell adapter.

Portal aliases clone the projected player cell. Live maps, replay maps,
inspection backdrops, retained redraws, local effect actor repaint and cached
plane transitions consume that cell rather than reconstructing player markers.
Neither the CPU painter nor simulation needs a glyph-specific branch.

The final player/body glyph is fixed after appearance-profile resolution. Old
profiles remain readable and unchanged, but their player/default glyph override
cannot replace `@`. Existing color, background, emphasis and animation settings
remain in effect. Creature and ally glyphs, ally orders, creature ranks, Half
Here's `h` annotation, corpse/status markers and hidden-actor privacy are not
changed. In particular, a separate allied clone is not recategorized as the
actual player.

The Appearance studio shows a read-only row, “Player glyph is fixed to @”,
instead of an editable value and adjustment/reset buttons. Mouse, keyboard,
and a stale value-edit focus cannot change that property. Historical JSON and
preset import/export remain compatible; player color/background/emphasis and
map font controls remain editable. There is no profile migration.

## Complexity

For `A` already-admitted actor cells and `P` projected player/body/portal cells,
this adds one constant-time identity selection per actor-cell construction and
removes the constant-size player mode-marker allocation. Each emitted fixed
player glyph uses one byte plus the existing `String` allocation; extra live
scratch memory is O(1) per cell, with O(P) existing output cells. It adds no actor
scan, world/visibility query, sorting, pathfinding, saved state, persistent cache
or RNG access. Existing profile resolution, disguise filtering, observation,
lighting, clone and alias costs are unchanged and are not claimed O(1).
The settings guard compares one fixed-length path per selected control and
adds O(1) work/memory per input or painted settings row; existing filtering and
profile serialization/import costs remain unchanged.

At native paint time the existing font/mask caches now rasterize/tint one `@`
glyph and no superscript for the player. Cold glyph rasterization still depends
on outline/bitmap area, warm painting on covered native pixels, and framebuffer
upload on drawable area. Removing a marker does not add a cache invalidation or
an extra frame. These are asymptotic bounds, not measured performance timings.

## Verification

Focused deterministic tests cover:

- Avatar, Blessed and Unblessed across revisions 29, 34, 43, 50, 51 and 52,
  including terminal-player cells, with serialized-state equality.
- Explicit player and global glyph overrides, retaining cosmetic styling and
  preserving the original saved player/profile values.
- Unchanged compound creature glyphs, ranked ally order annotations and Half
  Here clone markers.
- Both Dreamwalking endpoints, creature list and sleeping-body inspection.
- Portal aliases and hallucination phase changes.
- Legacy replay refresh, stepping, restart and profile reload.
- Read-only settings across mouse, keyboard, repeated attempts and stale focus,
  unchanged historical export data and independently editable style/font fields.
- Actual native CPU raster at 1280×800 and 1920×1080, including inspection
  backdrops, with a comparison proving that a synthetic old `3` marker would
  produce different pixels.

`CORPSE_PLAYER_GLYPH_REVIEW=<directory>` enables seven native PNG exports from the
focused tests. These exercise the shipped CPU painter, not an OS screenshot;
Windows display/input/GPU behavior is not claimed by these captures. Frozen
source scenes and their unrelated terrain/creature glyph parity are preserved;
only a player-specific old-presentation expectation may be narrowed if a legacy
oracle actually conflicts with this explicit instruction.

Validation results are recorded after the coordinated native test run.
