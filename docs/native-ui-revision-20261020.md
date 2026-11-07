# Native UI revision — 20 October 2026

## Designer changes

- Main title is `CALL OF THE CORPSE GODS`. Removed slogans, offline/account claims, preview footers, and duplicate keyboard instruction blocks from normal menu/start surfaces.
- `Continue` follows `Descend`, disabled when no valid live run exists. Existing live progress takes precedence over a previously loaded save. Hover uses the same bounds as click; selected actions invert to dark ink on pale paper.
- New-run form is reduced to `Descend?`, `Seed`, `Vessel`, `Descend`, and minimal Reroll/Back controls.
- Native rendering uses drawable-size pixels rather than enlarging a 1280×800 raster. The logical layout remains fixed and aspect-fitted to retain a full stationary dungeon view and existing controls. Windows carries a per-monitor DPI manifest.
- Ability summaries use measured ellipsis and avoid repeated reaction/fortune costs. Scrolling no longer changes the total ability count.
- The supplied pixel logo is the window and executable icon. All81 creature illustrations are embedded; the downloadable package requires no art installer, separate art ZIPs or network fetch.

## Art distribution

Original PNG masters are unchanged. Embedded WebP distribution copies retain original dimensions and use quality90 compression. They are not pixel-identical copies; comparison details and regeneration instructions are in `native-embedded-art.md`. All81 entries are covered by decode/index/dimension regression tests.

## Verification

Final build, test and actual desktop evidence is recorded in `NATIVE-VERIFICATION.md`. Automated render-size tests establish raster resolution and coordinate mapping; Linux desktop captures establish observed interaction. Neither substitutes for running the final Windows binary on Windows.

No simulation or balance change is part of this revision.
