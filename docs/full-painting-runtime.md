# Full painting restored — revision41 performance trial

The user chose full painting after the retained-rendering trial's mixed results.
The prior source, benchmarks and recovery archive remain preserved. This changes
presentation only, not gameplay, saving, compiled appearance or native Future.

## Requested frame path

The main event loop requests a redraw for world-dirty or cosmetic-dirty state,
or an eligible timed animation. An unchanged idle frame is not painted merely
because the loop runs. World-dirty state rebuilds the gameplay View; cosmetic
hover/selection redraws reuse the existing cached View. These are separate costs.

On a requested gameplay frame:

1. `Painter::game_at` clears the software RGBA buffer and paints the complete
   game composition: Chronicle, known map layers, creatures and resource/ability UI.
2. Effects are painted, then the hover tooltip when permitted.
3. Inventory, creature/tile inspectors, prompts, ability detail, blessing/debug
   UI and the Chronicle shutter are painted in their existing order.
4. The save-error banner is painted last.
5. SDL receives one full `texture.update(None, pixels, width * 4)`, then a canvas
   clear/copy/present. The loop retains its existing8ms sleep.

Window exposure/restoration requests a cosmetic full redraw; device reset also
recreates the streaming texture. Resize/profile changes retain their existing
geometry/appearance invalidation. No retained clean framebuffer, Scene index,
rectangle discovery, base restoration or partial upload runs in production main.
Full painting also skips the retained-only per-cell glyph/selection footprint
calculation when no damage scissor exists; normal raster clipping remains.
The experimental retained renderer and comparison tests remain available as
non-active evidence, not as the production rendering policy.

## Costs and bounds

Let P be native framebuffer pixels, C the detached paint cells, G the aggregate
glyph-mask pixels visited, T the text/layout work, and A the total painted layer
area including overdraw. Clearing is O(P), with4P bytes for the ordinary RGBA
buffer. Raster work is O(C + G + T + A), not just the count of changed pixels.
Recent Chronicle wrapping retains its existing bounded-text worst-case quadratic
metric work; its fade is O(H * (256 + row width)). Whole-frame upload transfers4P
bytes per requested redraw. Driver/GPU copy/present and any synchronization wait
are separate wall-time costs and do not have a fixed latency guarantee.

The extra retained renderer4P-byte clean base and new-View BTree/descriptor index
are no longer allocated by production main. Existing glyph caches, View, SDL
texture and prepared inventory tooltip data are still retained. Full painting
can repaint unchanged pixels, including game pixels subsequently covered by UI.

Most cosmetic frames use already prepared colors. Existing custom global
material/light animation may still rebuild cosmetic terrain appearance from the
cached TerrainObservation before rasterization. This is not a new simulation,
visibility observation or dungeon generation pass. Reduced-motion and eligible
animation scheduling are unchanged.

## Evidence boundaries

The frozen V3 same-binary normal small-s comparison measured free-backstep paint
at5.087ms full versus7.926ms retained, and command+durable-save+View+software-paint
at11.405ms versus14.732ms (medians,n20). These do not include SDL input, texture
upload/present or Windows runtime. They motivate the requested rollback, not a
claim that every full frame is faster. New rollback gates/timings are recorded
separately. Stable releases and the delivered game are not replaced by this source
change; remaining mechanics/layout release decisions are still pending.
