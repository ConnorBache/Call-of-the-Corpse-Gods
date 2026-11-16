# Ground-marker presentation follow-up

Timeweave uses three straight, antialiased strokes with equal screen-space axes for diagonal arrows. Arrows remain knowledge-filtered and paint from cached directions only. Numbered Non-Euclidean ground uses two-thirds-size glyph bounds and the same lower padding as grass. Animated material restoration preserves that alignment. Speed-up number appearance is unchanged. The player is painted only at their actual location in linked space; enemy/item linked aliases and ordinary portal rendering are unchanged.

No simulation, RNG, save format, visibility, movement, or time-travel rules change.

Complexity: each arrow paints a bounded native tile rectangle of A pixels, testing three fixed segments: O(A) time and O(1) scratch per arrow per repaint. Number glyphs use the existing font raster/cache path at reduced dimensions (cache-miss raster cost unchanged in kind, O(glyph area)); no extra world query. Removing player linked aliases removes its former topology lookup and alias allocations. Tests cover eight directions at three DPI scales, unchanged knowledge gates, one player image on repeated linked-space positions, and restored ground alignment. Native frame pixel tests additionally compare marker sizing/baselines.
