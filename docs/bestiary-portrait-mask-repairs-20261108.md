# Bestiary portrait alpha repairs, 2026-11-08

The original PNG masters and historical distribution WebPs are unchanged. These five targeted repairs use their original color pixels and manually reviewed spatial masks; no art was generated or redrawn.

- **Hexacorn (horn-6):** Restored 28,236 erased dark anatomy pixels through chest, shoulders and leg joints. Genuine horn, tail and inter-leg gaps remain.
- **Ursa Minor:** Restored 28,113 erased neck, chest and belly pixels. The exact pure-black underside boundary is visually inferred from bordering authored fur; no new RGB colors were invented.
- **Devil's Samara Tree:** Cleared reviewed upper-canopy negative spaces. Dark bark, trunk hollows, lower-root ambiguities and detached seeds remain.
- **Heptacorn (horn-7):** Cleared reviewed open mane/tail curl pockets while preserving dark purple hair, seven horns, fine strands and hooves.
- **Valta:** Replaced the thick opaque surround with translucent luminous glow. Reviewed interior masks protect dark anatomy, including the far-hind-leg attachment. Changed glow pixels are unmultiplied from black so RGB × alpha reconstructs their original black-backed source within one byte. Retained pixel support is exactly unchanged. Ordinary one-pixel edge-reconstruction validation is not relaxed for other assets.

`tools/creature-alpha/reviewed/*-mask-review.json` records the geometry or component selections. Approved RGBA, alpha masks and hashes are bound by `reviews.json`. The Valta exception additionally binds the protected-anatomy, support and correction masks and a fixed reconstruction rule. The five PNGs were re-encoded using the existing WebP q90/alpha100/method6 settings; alpha and dimensions must match exactly.

## Verification

- Source and legacy files untouched; other 79 canonical/runtime assets and manifest rows unchanged.
- Full-resolution four-background review (white, red, gray, black), plus independent visual review of all five targeted candidates.
- `python tools/creature-alpha/pipeline.py verify` checks all 84 source/runtime hashes and review gates.
- `python tools/creature-alpha/test_targeted_repairs.py` checks source RGB, fixed anatomy/gap points, all 84 WebP alpha/dimensions, Valta support/reconstruction, and rejection of an unreviewed recolor.

The status is deliberately **reviewed-targeted-alpha-repair-needs-manual-edge-review**, not universal artist certification of every tiny rim or ambiguous dark boundary. The original 84-portrait overview audit and native R51 baseline review found these five definite mask defects. Current-R52 aggregate tests and Windows/native runtime validation must be run by the integrating build; these asset-level checks do not claim that validation.
