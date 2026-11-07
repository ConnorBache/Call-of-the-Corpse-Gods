# R50 corpse, blood and smaller-dialog visual QA

Reviewed all 27 native PNG captures in this directory. No Cargo commands or live
source edits were made during this review.

## Findings

- Corpse: the pale outlined X is visible over the retained dead-creature glyph.
  Adjacent living, living-heartless and sleeping-body glyphs remain distinct;
  only the heartless living actor has its ring. The fog capture hides the corpse
  and reports zero visible creatures.
- Blood: the default three blood-colored dots remain ground punctuation, without
  a filled tile. The custom-foreground capture changes those dots to the chosen
  teal while retaining the floor, corpse marker and other glyphs.
- Dialogs: all twelve before/after pairs were inspected at 1280×800, 1920×1080
  and 960×720. The 84% sheets are visibly smaller than the 88% control. Inspection
  content, hazard text and Yes/No controls fit. Long ability text shows FIRST LINE
  at the top and FINAL LINE at the bottom, with scroll/close instructions intact.
  Chronicle, sidebar and gameplay composition remain unchanged.
- **One cosmetic regression needs correction:** the inspection sheet loses its
  bottom border at 1280×800 and 960×720 after the size reduction. The border is
  complete at 1920×1080. Exact border-color row checks confirm this is real, not
  image display scaling. Fractional rounding places the minimum-width bottom
  stroke outside the inspector's own clip.

## Staged correction

`staged-fix/border-footprint.patch` clamps border strokes inside their rounded
native panel footprint, adds a five-resolution self-clipping/perimeter test and
updates the complexity note. Patch application passed `patch --dry-run -p1`.
It has **not** been applied, compiled or executed by this review. Integration and reruns were pending at the time of this review. Recapture the inspection pairs after applying it.

## Executed evidence from the test run

`../focused-desktop.log` records passes for all three new visual tests, three
corpse-marker tests, one blood-dot sequence test and two compact-dialog tests.
Those tests additionally verify invisible/forgotten corpse exclusion, hidden
actor invariance, custom foreground preservation, dry/bloody background equality,
full/retained/tile parity, read-only Run state, high-DPI input/damage transforms,
and exact base-frame restoration after repeated dismissal. Screenshot review
alone does not establish these behavioral properties.

That initial desktop run ended at 587 passed, 1 failed and 32 ignored. The failure
is the separate debug inventory fixture's `-0/5` versus `0/5` category text, not a
corpse/blood/dialog test. This report is not a claim that the entire suite passed.

## Limits

The images use controlled rendering fixtures, not a campaign playthrough. Their
old fixture ability IDs and inherited “incomplete port” provenance text do not
represent an ordinary generated R50 starting loadout. Before images are freshly
painted using the test-only former native transform, not resized screenshots.
The review does not verify physical mouse input, SDL window interactions,
physical-GPU output or Windows launch behavior. No mechanics were changed here.
