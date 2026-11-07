# R51 grass and amulet display

Grass retains its green double-quote identity and paints its ink at the lower edge
of its tile, with one logical pixel of padding. Only grass's glyph ink moves.
The tile background, edges, overlays, terrain fixtures, flowers, snow, actors and
other glyphs keep their existing geometry. Explicit custom grass glyphs and colors
remain authoritative. Remembered grass uses only the last observed terrain.

Amulets display `?` instead of the old double quote. The map adapter supplies the
new identity for both current and remembered/detected items without rewriting
serialized items. The two editable built-in amulet appearance entries also show
`?`. Imported inherited identities adopt the display default, while explicit
appearance overrides, including an explicitly chosen old quote, are preserved.
Pack and inspection currently show item names/descriptions rather than glyph
icons; they do not require a separate glyph substitution. Frozen v48 reference
fixtures are unchanged. The enumerated appearance-fixture derivation adds only
the two newly authorized default fields when the input omitted them.

## Complexity

Per observation, the grass marker adds O(1) time and one boolean to each detached
cell, over the existing O(N) terrain projection for N observed cells. Amulet
selection adds one fixed item-kind match to the existing item projection. No
simulation, save, temporal history, visibility or gameplay RNG work is added.

Let W and H be cached native ASCII mask dimensions and R its nonzero row runs.
The existing O(W*H) cache construction and O(H+R) metadata now record the last
covered row in O(H) time and O(1) additional memory, once per cache insertion.
Every warm bottom alignment is O(1), with no new mask cache key or cache entry.
The cache still has its existing 512-entry clearing bound. Compound-font masks
already store cropped height and need only O(1) placement arithmetic. Existing
font rasterization, glyph coverage painting, GPU mask upload/cache misses and
framebuffer work retain their documented costs. Backgrounds and damage bounds
are unchanged. No additional world query or per-frame mask scan is introduced.

## Verification

Deterministic `grass_amulet` tests cover green defaults, hidden/remembered cells,
fixtures/flowers/snow, custom appearance and imported legacy identities, read-only
item projection, native-pixel ink translation and mask reuse at three drawable
sizes, GPU recording without CPU writes, and full/retained/tile repaint parity.
These checks and the native workspace suite must be run before release. Runtime
performance timings and Windows-native presentation remain separately unverified.
