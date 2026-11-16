# Corpse identity colors and larger red crosses

The latest designer request is: “Make the X over corpses larger and red, and make the corpses retain the color they had in life.” This replaces the earlier small, pale cross presentation.

## Behavior

- Corpses now resolve their saved creature glyph and color through the same creature appearance entry used while alive. Individual saved color variants remain exact source colors, and customized creature colors/glyphs apply consistently to living and dead instances. The generic corpse appearance entry no longer replaces that identity.
- The separate death X is red (`#f53640`) with a thin dark edge. Its diagonal endpoints span 60% of the shorter tile dimension, compared with approximately 19% of the tile width previously. The stroke radius is 0.48 logical pixels at the standard 15-pixel tile width. Enough surrounding glyph remains visible to retain its identity color.
- Both ordinary map painting and effects repainting use the same red source color and existing local display illumination. Existing dream-plane treatment remains intact. Heartless rings keep their previous color and geometry. Default black Dire Bear identity outlines remain available on corpses too.
- Saved/profile RGB identity is retained, not the exact final framebuffer RGB. Existing corpse opacity and contrast remain lower than living creatures because corpses keep their non-important readability classification. This removes the generic lavender tint without changing scene illumination.
- Visibility admission, corpse/item/fire stacking, projected disguises and hallucinations, dream-plane knowledge, simulation, decay, death processing, resurrection and all time-travel paths are unchanged.

## Glyph-center correction

The subsequent request, “The X needs to be centered on the glyph,” is implemented by attaching the same larger red X to the actual native glyph ink bounds instead of the map-cell center. See [centering behavior, cache costs and regressions](revision52-corpse-cross-centering.md). Living colors, outlines, opacity and save compatibility below are unchanged.

## Save compatibility

No core or save-schema change is needed. `Actor.color` already survives ordinary death, generated corpse creation, serialization and revival. The previous lavender corpse tint was applied only by desktop observation. Old saves use their existing stored actor color; no species palette lookup is used to invent an unknown historical individual color. Existing loader migrations retain their existing behavior.

## Complexity and portability

Let A be the already-admitted actors, T the native tile area and P the clipped native cross bounding area. Corpse observation still performs the existing creature-profile lookup and constant-size RGB conversion per admitted actor. It introduces no world scan, lookup cache, RNG call or persisted field. The profile resolver has its existing bounded profile/catalog lookups and small settings clone; there is no new asymptotic cost.

Each corpse cross is rendered once per painted corpse layer, including relevant effect repaint layers. The native rasterizer is O(P × 16) time and O(1) scratch, with the same fixed 4×4 antialias coverage samples per pixel as before. P ≤ T; a larger cross increases the constant number of touched pixels. GPU command recording remains one existing native-pixel command per covered pixel, O(P) output, while CPU painting updates the clipped framebuffer. No timing improvement is claimed. Runtime full-frame or damage-layer scheduling is unchanged. The RGB identity and marker geometry are detached presentation inputs, independent of core state and portable without simulation changes.

## Verification

The coordinated compile21 test binary passed all 17 focused checks: eight corpse-marker/color tests, two existing corpse/visibility regressions and seven projected-identity/privacy regressions. The tests cover actual engine death, old/current save round-trips, revival, individual colors, custom creature appearance, red cross extent, retained/tile rendering, native-pixel clipping, GPU command recording, and hallucination privacy.

All four native CPU-framebuffer captures were visually inspected: paired living/corpse variants from revisions 29 and 52 at 1280×800 and 1920×1200. The upper row is alive and the lower row is dead. Left to right: individually teal-colored rat, Dryad, Hemogoblin, Dire Bear and Vampire Bat. The larger red crosses are distinct from the retained teal, green, red, legacy orange or current black-with-outline identity ink. Existing lower corpse opacity/contrast remains visible. No generic lavender corpse tint remains.

Evidence is in `verification/r52-corpse-colors-cross/compile21/`, including passing logs, four original PNGs and source/binary/output hashes. Earlier fixture-only failures are preserved separately: invalid test geometry, invalid substrate/log pairing, then an out-of-sight legacy vampire. The final fixture passes normal save validation and asserts actual direct visibility for every captured actor. No validator or visibility rule was bypassed. Optional screenshot output uses `CORPSE_COLOR_OUTPUT`.

These are actual native software-renderer pixels and GPU command-recording checks, not Windows execution or hardware-GPU readback certification. Full-workspace and platform/runtime gates remain integration requirements.
