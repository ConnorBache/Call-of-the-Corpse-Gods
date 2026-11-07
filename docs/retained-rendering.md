# Retained software rendering — implementation under verification

User-directed reuse of unchanged framebuffer regions. Conservative rectangles can repaint unchanged pixels within their bounds. Full pixel-oracle and platform results must be reported separately from source design.

API: `render::RetainedGame::{update,restore,invalidate}`, `render::damage::{Rect,Damage,upload_source}`, and Painter's independent `damage_clip`, `logical_damage_rect`, `raster_stats`. Main/pack integration expands every intersecting translucent overlay to its complete contrast-dependent line/overlay bounds, restore old and new footprints from the clean game base, and replay overlays in their original order. Reset `damage_clip` after each frame. Empty damage means no raster/upload; ordinary empty frames skip presentation, while exposure explicitly re-presents retained texture. Full invalidation is mandatory for font/palette/profile/layout and screen changes. Resize is also automatically detected.

Discovery inspects detached View paint descriptors only on a new View: O(C log C + copied descriptor bytes + spatial bucket entries), not world state or framebuffer pixels. Spatial 32-native-pixel buckets identify intersecting cell-stack indices in O(bucket visits + K log K), preserving original draw order. Damage insertion/coalescing has a fixed maximum of eight rectangles. Actual overlaps conservatively merge; touching rectangles merge only if their union adds no unchanged area, so an L-shaped or corner contact does not fill its empty center. A ninth fragment chooses the deterministic least-added-area pair plus overlap closure, preserving disjoint alpha writes. Ordinary insertion is O(R²), R<=8; overflow selection is O(N⁴), N<=9, with bounded scratch. This is not a minimal rectangle-union algorithm, and true overlaps/capacity merges can still repaint unchanged pixels. Damage raster and row copies scale with selected primitive footprints and Q damaged pixels; overdraw inside a damage rectangle is intentional. No unchanged framebuffer scan or per-frame full buffer clone. Extra retained framebuffer is exactly 4P bytes, plus paint descriptors/spatial metadata. Counters count raster operations (including overdraw), not unique changed pixels. Upload helper returns a borrowed subregion with full framebuffer pitch, no prefix or rectangle buffer copy.

Current custom whole-terrain animation and Dreamwalking retain existing descriptor rebuild behavior when map damage requires it. They can cause dense map damage; this first implementation does not claim O(animated cells) for those existing global transforms. Canvas full-texture copy/present remains a separate operation; no assumed preservation of the SDL backbuffer.


## UI composition and complexity

- Pack paint state clones only small UI selection/request metadata and prepared
  tooltip text, not item groups or a Run. Content/layout generations cause a full
  panel invalidation; hover-only changes invalidate old/new row/button/tooltip.
  Exact same hover returns empty damage. Prepared tooltip wrapping is reused by
  paint, not repeated for discovery and drawing.
- Main retains original z-order. For every bounded disjoint dirty native rect,
  it restores clean game-base pixels, then recomposites intersecting overlays.
  Unknown complex modal states use explicit full-frame fallback. Menu/replays/
  settings and other screens retain existing full painting and invalidate game
  retention; resizing invalidates both renderer and overlay state.
- Map hover conservatively covers the tooltip's complete possible vertical
  footprint because placement flips above/below according to its wrapped height.
  It is NOT falsely advertised as exact minimum damage. A changed/underlying
  translucent tooltip expands to that complete footprint for contrast sampling.
- Game-base changes underneath an opaque overlay are still recomposited. We do
  not suppress them after the renderer has overwritten final pixels with base;
  doing so would corrupt the retained final framebuffer. Later occlusion caching
  would be a separate measured change, not hidden here.
- SDL uploads each native rectangle from a checked borrowed pixel slice with the
  full framebuffer pitch; no full-frame byte copy or pixel-diff scan. Empty total
  damage skips software painting and texture update; ordinary empty frames skip
  present too, while window expose/restore explicitly re-presents the valid retained
  texture without uploading or painting pixels. GPU canvas copy/
  present remains the ordinary full texture operation when there is damage.
- Optional timing labels separate retained composition, texture upload, present.
  Their enabled collection must be set by a harness; no performance claim yet.

Complexity: pack descriptor O(I + tooltip text/wrapping), changes O(small hover
regions + tooltip text); effect projection O(C + E log E) including a known-cell
bitmap and existing effect tile enumeration. Overlay planning has at most eight
coalesced native rectangles; composition visits each intersecting overlay for
those bounded rectangles, with normal text/layout work and clipped raster cost.
Thus it is not a blanket O(changed pixels) claim. Retained base adds 4P bytes in
renderer, plus small descriptors and effect projections. No simulation/RNG/save
policy changes.


## Validation contract

Compare complete native buffers against full rendering at fixed visual times for movement, glyph overhang/order, HUD changes, tooltip movement/removal, pack selection/scroll, translucent shutter opening/closing, sparse/dense animation, profile changes and resize. Expose without damage re-presents the valid texture without software repaint/upload; texture recreation requires full upload. Test-only raster counters count clipped pixel operations, not unique changed pixels or every upstream coverage sample. SDL upload/present timing and Windows runtime require independent execution.
