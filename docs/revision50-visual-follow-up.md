# Revision 50 visual follow-up

## Scope and interpretation

The request for X marks on corpses and blood-colored dots is already implemented
in the retained R46 renderer, with no enable switch missing. The existing X is
drawn over the observed dead actor's retained glyph, using a pale stroke and dark
outline. Living heartless actors retain their ring, and living sleeping bodies
receive neither dead-body X nor heartless ring. Existing observation admission
and layering are retained: hidden actors are not revealed, and a corpse remains
under living actors, items and fire. No new corpse-memory system was introduced.

Floor blood recolors existing floor punctuation with the authored blood color.
The same dry and bloody tile has identical background bytes. A user-selected
blood foreground remains effective; blood background/glyph overrides cannot
turn a dot into a filled tile or an X. Bloody walls, contaminated water, Ichor,
Dreamwalking ink and their simulation/appearance rules remain separate.

“Dialogue smaller” is interpreted as the existing modal dialog sheets, following
the R46 dialog terminology. Their shared linear scale changes from 0.88 to 0.84:
4.55% smaller than the existing sheets, or 8.88% less screen area. The single
constant is reversible. The authored content, wrapping, scrolling and hit areas
stay in the same logical coordinate system. Input and dirty-region conversion
use the same new transform. The Chronicle log, gameplay canvas, original theme,
Pack's separate bounded layout and nonmodal hover tooltips retain their scale.

## Deterministic verification and native captures

Existing `corpse_marker`, `blood_dot_sequence` and `compact_dialog` regressions
remain. The compact transform test now checks the exact scale ratio at five
drawable sizes, alongside high-DPI hit conversion, damage and canvas restoration.

`visual_requests_revision50_tests` adds:

- An R50 rendering fixture with dead, living, heartless and sleeping bodies, plus
  real observed floor-blood cells. Default and custom blood foregrounds are
  checked without changing their floor backgrounds or glyphs.
- Full/retained/tile rendering equality after blood appears; read-only Run checks.
- Invisible corpses, wall-occluded corpses, forgotten terrain and hidden actor
  death/identity changes do not reveal a corpse marker.
- Native before/after dialog painting at 1280×800, 1920×1080 and 960×720, including
  inspection, hazard confirmation, and long ability details at top and bottom.
  Each close/reopen restores the original Chronicle, HUD and map buffer exactly.

Set `CORPSE_R50_VISUAL_OUTPUT` to an absolute output directory, then run:

```sh
sh tools/cargo.sh test --locked -p corpse-desktop visual_requests_revision50_tests -- --nocapture --test-threads=1
sh tools/cargo.sh test --locked -p corpse-desktop corpse_marker -- --nocapture --test-threads=1
sh tools/cargo.sh test --locked -p corpse-desktop blood_dot_sequence -- --nocapture --test-threads=1
CORPSE_COMPACT_DIALOG_OUTPUT=/absolute/compact-dialogs sh tools/cargo.sh test --locked -p corpse-desktop compact_dialog -- --nocapture --test-threads=1
```

Before captures reconstruct the former 88% native transform in test-only code
and paint the exact same content as the new 84% captures. They are not rescaled
images. Existing GPU matrix coverage includes corpse marks and blood dots;
software/offscreen capture does not establish physical-GPU or Windows behavior.
Execution and visual inspection results belong in the current verification
report, not inferred from these authored tests.

## Complexity

The modal scale change is one constant. Native border strokes are clamped
inside their rounded panel footprint so fractional scaling cannot place a
minimum-one-pixel bottom/right stroke outside the panel's own clip. Border
painting remains O(native perimeter pixels) time and O(1) extra memory; it
continues through shared CPU/GPU and dirty-region clipping. Modal input/damage
transformation remains O(1) time and extra memory per conversion; painting retains existing content,
font shaping/rasterization, clipping and pixel costs. Native glyph cache misses
depend on outline complexity and raster area, not constant time. Dialog content
layout and scrolling retain their prior text-length and wrapped-line costs.

Corpse and blood production code is unchanged. Each admitted corpse X costs
O(Pt × 16) fixed coverage samples and O(1) scratch for Pt clipped native tile
pixels. Existing actor visibility/plane queries precede it; this is not a claim
that scene observation is constant time. Blood material selection is bounded
per already observed terrain cell, with existing lighting/material work retained.

Tests use O(P) buffers per renderer and O(R) cloned fixture state, where P is
native framebuffer pixels and R is the fixture Run size. Capture encoding writes
O(P) pixels plus PNG library work/memory. These are source-derived bounds, not
measured performance results. No world-state, RNG, turn, combat, save or replay
mechanics are changed.
