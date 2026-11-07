> **October 26 alpha-pipeline update:** Runtime artwork now uses reviewed/frozen RGBA PNG assets and no runtime keying. Original masters remain unchanged. The WebP and earlier verification descriptions below are historical, superseded for asset consumption by `docs/creature-alpha.md` (or `creature-alpha.md` from this directory). They do not certify the new segmentation.

# Bestiary visual-composition revision

Visual-only revision of the native r31 bestiary. All 79 authored creature pages,
art source bytes, immutable prose, stats/mechanics, controls and saves remain
unchanged. The original r31 Windows package is unchanged; this archive contains
updated source, not a newly built Windows executable.

## Changes
Smaller protected title; occupied-silhouette-responsive specimen layouts;
continuous readable history; grouped abilities/locations for the longest entry;
wide-creature spanning layout; restrained secondary text; contained same-hue
organic contrast backing only for dark void creatures. Body text is 18–20px in
all 79 resolved pages. Original art proportions and occupied RGBA pixels remain
intact. See docs/native-manual-layout.md and docs/complexity-ledger.md.

## Verification scope
Focused final manual tests: 29 passed, 0 failed, 1 existing ignored benchmark.
Coverage includes all 79 resolved layouts, full authored text, occupied-art
pixel preservation, protected UI regions, palette contrast, stats and controls.
Historical NATIVE-VERIFICATION.md and INTEGRATED-VERIFICATION.json describe the
original r31 checkpoint, not a newly verified Windows build. SOURCE-MANIFEST.json
is regenerated for this archive; the original manifest is retained under
verification/original-r31-source-manifest.json.

The included render manifest records all 79 actual 1280x800 after captures and their
PNG/pixel hashes. Images are delivered separately.

Independent final full-workspace QA: 484 passed, 0 failed, 1 ignored across
107 result blocks; exit code 0. Test optimization level 2, incremental disabled.
All 18 varied-resolution capture checks passed. These checks used the exact
frozen candidate on Linux; they do not verify Windows runtime. The existing
Windows package is unchanged. Warm-paint benchmarking is separate and is not
a correctness or aesthetic-quality claim.

## Build
Use pinned Rust 1.99.1 and the SDL2 development/runtime prerequisites documented
in README.md and docs/windows-cross-build.md. Run cargo test --workspace and
cargo build -p corpse-desktop. Preview an exact page with:

    corpse-desktop --manual-entry river-drake --render-test river-drake.png --render-size 1280x800

Source files and Cargo.lock are included; target/, toolchains and prototype
experiments are excluded. ZIP entries use fixed timestamps and sorted paths
for reproducible packaging. Only manual.rs and the two layout/complexity docs
differ from the preserved source, aside from this archive's verification notes.
