# Bestiary continuous colors

## Requested presentation

The Bestiary previously selected either pure black or pure white text by comparing
contrast ratios. That produced a one-page polarity jump near floor 17. Secondary
text independently selected among nine contrast steps, and unread exclamation
marks stayed fixed yellow. The void specimen backing also appeared at one
luminance threshold and changed the art inset from zero to twelve pixels.

This patch retains the solid parchment/red/black page anchors at floors 1/25/40.
Every changing palette field now uses explicit piecewise-linear interpolation
at fractional visual depths, quantized only at the final 8-bit channel output.
Text approaches white evenly by floor 17 instead of swapping in one step.
Secondary text then eases to pale gray by floor 40. The unread mark moves through
warm amber, white, and gold. Selected labels and marks have separate continuous
light anchors because their actual background is the changing selection bar.
No creature RGB, alpha, source master, lore, statistics, discovery data, simulation
or save behavior is changed.

A continuous dark-to-light foreground necessarily passes through the background's
luminance. A thin glyph keyline preserves an identifiable edge through that crossover.
Ordinary text starts with a page-colored keyline, retaining its original
first-floor weight, and its keyline evenly darkens to black by floor 5. Selected
labels and unread markers keep a black keyline throughout. There is no
threshold-triggered inverse-outline switch. It uses ordinary cached glyph commands, so
it does not add an unsupported GPU command. It must still be visually reviewed
at small label sizes and native drawable scales before release.

The void field is invisible against the page through floor 19 because its RGB
is exactly the page RGB. It then interpolates to the existing burgundy color by
floor 29 using the same stored splash alpha. The zero-to-twelve-pixel inset is
interpolated across that interval, with final integer-pixel quantization.
There is no opacity-threshold activation or portrait recoloring.

## Verification and explicit limits

New tests in `bestiary_theme_tests.rs` cover dense fractional-depth continuity,
anchors, unread markers on selected and ordinary rows, invalid preview depths,
void-field/inset continuity, and supported glyph-command capture. Existing
layout, authored-pixel preservation and discovery tests remain applicable.
The former text-contrast test now checks the actual fill/keyline combination,
not the mathematically impossible requirement that a continuous polarity-flipping
fill alone always contrast with the page.

`CORPSE_BESTIARY_THEME_OUTPUT=/path/to/output cargo test -p corpse-desktop
bestiary_theme_tests -- --test-threads=1` exports transition pages at 1280x800,
1920x1080 and 900x1200, including selected/unselected unread markers and void
portraits. They require visual review, especially small text around floors 7–9
and 16–18. All Bestiary tests and the full workspace suite must be run on the
fully recovered source before shipping. Historical screenshots are not current
verification. Windows/native-monitor testing is separate.

At patch preparation, latest-source restoration remained incomplete. Four pure
palette tests extracted directly from the patch passed under Rust 1.99.1, but
the full native application was NOT compiled and renderer/capture tests were
NOT run. A separate Python math model sampled 390,001 fractional depths, with the actual
page/selection-bar background used for each foreground. It checked a 4.5:1
minimum for at least one of the fill or actual keyline color relative to that
background. This is palette evidence, not a WCAG certification of antialiased
text or native renderer correctness. The independent palette preview is labeled
as such and must not be described as a game screenshot.

Subsequent isolated native validation used the complete R51 baseline with only
this theme treatment, not the incompletely recovered R52 game. Its Bestiary
suite passed 38 tests, zero failures, with one timing test initially ignored.
The timing test was then run explicitly and passed. Three embedded-asset tests
also passed. This produced 82 portrait pages and 66 transition captures across
three resolutions. First-floor keyline rendering was byte-equal to ordinary
text at six font sizes, both weights, and all three resolutions. All baseline
page layouts remained in bounds. The middle-depth dark edge is visibly an
outline, especially on small gray type; this is an intentional readability
tradeoff, not visually unchanged typography. Native current-R52 integration,
full-workspace tests and physical Windows/GPU verification remain pending.

One shared-cloud warm-paint sample measured 10.666–16.308 ms at 1280x800 and
17.470–21.579 ms at 1920x1080 across four selected pages, 24 frames each. These
are current baseline-preview timings, not an old/new speed comparison or a
Windows performance guarantee.

## Complexity

Let C be displayed text characters, G the total native raster glyph coverage,
P the selected portrait pixel count, S the ink-splash pixel count, and F the
framebuffer pixels. Each depth-theme evaluation is O(1) time and space. It uses
fixed color anchors and has no loops over gameplay state or wall-clock time.

The four keyline passes plus the normal fill use five cached-glyph
passes per text span: O(C + G) warm work with a five-pass constant factor, not an
unchanged-performance claim. Cold glyph rasterization remains the existing
fontdue/cache cost; no new font, per-depth glyph cache, or persistent text buffer
is introduced. The normal no-scroll layout and its O(C) temporary text data are
unchanged. Existing framebuffer clearing and upload remain O(F).

Void-field recoloring remains O(S) on a changed color cache key and O(S) retained
RGBA memory; it may run at more fractional preview depths than before. Only one
field is retained. The unchanged creature decode/trim path is O(P) cold work and
space, with no new segmentation or per-frame portrait-pixel scan. Warm picture
compositing remains proportional to destination coverage. Native frame timing,
small-text readability, and physical-GPU/Windows performance remain unmeasured.
