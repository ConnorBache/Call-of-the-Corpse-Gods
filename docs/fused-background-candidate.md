# Fused opaque game background candidate

Integrated into the candidate source after review. Not yet built, tested or
timed. The independent test-only sparse ASCII control remains separate. No
release or speedup claim follows from source review.

The private `Painter::clear_with_rect` helper uses the test-only
`background_fused_enabled` flag, which defaults true. False calls the existing
clear followed by the existing rect. The eight-arm native paint matrix toggles
ASCII, background and instrumentation independently, preserving an all-off
algorithm reference. No production setting, cache or public API is added.

## Source and pixel proof

Current game_at performs full clear, then one unexplored-map rectangle before
other paint. Both are opaque. A destination pixel covered by the rectangle is
written twice before any consumer reads it. The new helper assigns its final fog
color immediately; other clear-selected pixels receive the base UI color.

The old clear ignores Painter.clip but respects framebuffer, native damage and
optional tile spans. The new helper iterates those EXACT clear-selected row spans.
The old rectangle rejects nonpositive logical dimensions, rounds each endpoint,
expands a positive stroke to at least one native pixel, then intersects with
native_bounds (including its independently rounded/minimum-one logical clip).
The new helper computes those same rectangle bounds once. Within each selected
clear row span, it writes up to three disjoint contiguous slices: base before,
fog inside, base after. Tile-mask restrictions already apply equally to both old
passes because they cannot change between these consecutive operations. Pixels
outside the selected clear set are never touched. All written pixels get alpha
255; no prior RGBA is sampled. Empty/offscreen/invalid rectangles still clear the
same independent base region, and empty/outside damage performs no writes.

No grouping of overlapping Cells is attempted. Individual tile backgrounds
already use `rect -> native_fill`, which bulk-writes the whole clipped native tile
as contiguous row slices; this new candidate removes a different redundant pass.

## Complexity, memory and evidence limits

P = native framebuffer pixels; Q = clear-selected pixels after damage/tile clipping;
A = old fog-rectangle selected area; H = rows in the clear damage bounds; S = selected
tile spans. Old work: O(H + S + Q + A), Q+A pixel writes. New work: O(H + S + Q), Q
pixel writes, at most three contiguous writes per selected span. In ordinary full
painting, Q=P, so the change is P+A writes to P writes. Both remain O(P) in the
full-frame bound. Extra scratch O(1); no retained memory or allocation. Existing
row_segments scan/lookup work remains included. Native pixels, not logical tiles,
are the cost variable. No font, alpha-mask, SDL/upload or animation work disappears.

Attribution already observed substantial clear and opaque-fill cost at 4K, but
that is motivation, not a speed claim. Existing Background span still encloses the
whole operation. Fused primitive work is attributed under FillOpaque rather than
Clear; compare total Background or total game paint, never Clear in isolation.
The raster operation counter deliberately drops by A, reflecting removed writes.
The new native helper has no per-pixel profiling branches or timers.

Correctness tests cover 1280, 1080p, 4K, odd dimensions, portrait and subpixel
canvases, both centered/responsive origins, arbitrary incoming RGBA, invalid or
partly offscreen logical rectangles, logical clips (including invalid/one-pixel),
native damage (including empty/outside/partial), and empty/full/disjoint tile masks.
They compare every framebuffer byte with old public clear+rect calls and assert
that fused visit/write count equals old clear's count. The full game integration
oracle checks frozen fixture pixels at 0 and 420ms. These tests are integrated, not
yet passed. Lead should also use its cross-build real-scene/Pack parity gate.

Performance comparison should hold ASCII sparse mode fixed, alternate the
background toggle with collector off for reported wall times, warm both arms,
include 4K and non-4K scenes, and compare Background and whole-paint costs.
A separate collector-on run can attribute the difference. Source-only audit has
not measured either candidate performance or end-to-end input/present latency.
