# R50 final corpse, blood and dialog visual QA

## Result

**Pass for the reviewed visual scope.** The fractional-scale border correction
resolves the only defect found in the first capture set. No further visual
changes are required by this review.

The inspection sheet now has complete borders at 1280×800, 960×720 and
1920×1080. Exact pixel checks confirm a continuous bottom stroke spanning 437,
328 and 590 pixels respectively. Inspection text and Close remain inside the
sheet. The corrected hazard and former-88% ability borders were also inspected;
no content, controls or scrolling text were clipped.

## Capture coverage

The original review inspected all 27 captures: three corpse/blood/fog scenes and
12 before/after dialog pairs spanning inspection, ability top, ability bottom
and hazard confirmation at all three resolutions. This final review compared
all 27 regenerated images with those reviewed originals and visually inspected
all seven changed images, plus the unchanged 1920×1080 final inspection sheet.

Only border pixels changed in those seven images. The other 20 images are
pixel-identical, including all corpse/blood/fog images and all final ability
screens. All PNG alpha channels are fully opaque. Per-file difference counts
and bounds are recorded in `recapture-pixel-comparison.json`; artifact hashes
are in `capture-sha256.txt`.

The reviewed images show:

- A distinguishable outlined corpse X over the retained dead glyph. Living,
  living-heartless and sleeping-body glyphs keep their separate appearances.
- No hidden corpse marker in fog; the visible-creature count is zero there.
- Authored blood-colored floor dots with unchanged backgrounds, and the chosen
  teal foreground in the custom-appearance case. Other materials are unchanged.
- Modestly smaller 84% modal sheets versus the native 88% controls. Long ability
  details retain FIRST LINE at the top, FINAL LINE at the bottom, and visible
  scroll/close instructions. Hazard text and Yes/No controls fit at every size.
- Unchanged Chronicle, sidebar, map, theme and nonmodal scale.

## Behavioral test evidence

`../workspace.log` records passing results for all four new visual tests,
including `revision50_dialog_borders_survive_their_own_fractional_clip`, plus
three corpse-marker, one blood-dot sequence and two compact-dialog tests.
These additionally cover invisible/forgotten corpse exclusion, hidden actor
invariance, blood preferences/background equality, full/retained/tile parity,
read-only state, high-DPI input/damage geometry and exact background restoration
after repeated dismissal. Border perimeter/clipping is exercised at five sizes.

The full workspace run recorded 2,004 passed, 1 failed and 38 ignored.
Its sole failure is the separate newly added legacy jewelry grant-label fixture;
all visual tests above passed. This report does not claim a clean workspace gate
or substitute for the subsequent fix and final rerun.

## Limits

These are controlled native-renderer fixtures, not campaign or Windows launch
verification. The synthetic fixture retains historical ability IDs and an
“incomplete port” footer; these are not representative new-run R50 metadata.
Before captures use the former 88% native transform with current border raster
correction, not rescaled images or the old executable. Physical mouse/keyboard,
SDL window behavior, physical-GPU and Windows runtime were not exercised here.
No source edits or Cargo commands were made during this final review.
