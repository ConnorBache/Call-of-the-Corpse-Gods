# Timeweave visual projection (revision41)

The existing terrain observation pass performs an indexed lookup for each emitted terrain cell. Only current, directly visible cells on the observation's current floor and waking/dream plane receive a detached `(dx,dy)` arrow. Invalid/non-adjacent endpoints are not drawn as invented connections. Remembered-only, sensed-only and hidden cells do not expose live trail state.

The cached `render::Cell` carries this direction. Animated terrain reconstruction copies it from the existing cached cell. Painting adds a static teal line and arrow beneath the terrain glyph; later item/actor layers retain their normal occlusion. No render-time Run access, trail traversal, visibility computation or simulation RNG is introduced. Dream presentation retains the explicitly teal ability marking.

For C emitted terrain cells and L trail segments, projection adds expected O(C) hash lookups (worst O(C*L) under pathological hash collisions), O(C) bounded detached metadata. Paint adds O(cw+ch) logical stroke samples per marked painted cell plus the existing native pixel scaling footprint; scratch is O(1). It does not add an idle animation or force an observation rebuild. Existing full-frame rendering and observation costs are unchanged.

Tests cover all eight adjacent directions, direct/current visibility gates, wrong floor/plane, old revisions, malformed/non-adjacent endpoints, and bounded teal arrow raster at1×/1.5×/2×. No Windows runtime verification is implied by software-buffer tests.
