# Creature foreground segmentation and alpha workflow

This revision changes **image assets and image consumption only**. Original PNG
masters, lore, statistics, page layout, gameplay, saves and RNG are unchanged.
There are **81 source plates / 79 bestiary pages**, not 81 creature pages.

## What ships

`assets/creatures-rgba/*.png` are actual lossless RGBA PNGs embedded by `build.rs`.
The renderer decodes their RGBA verbatim, then trims fully transparent outside
margins. **There is no runtime color key, perimeter flood, brightness filter,
CSS blend, alpha smoothing, or per-frame image cleanup.** Nearest-neighbor artwork
sampling remains unchanged. Build rejects a non-RGBA PNG.

`assets/creature-alpha-manifest.json` is the authoritative per-plate status and
SHA-256 provenance inventory. Its counts distinguish:

* `reviewed-partial-cleanup-needs-manual-edge-review`: **3 plates (Phoenix, Horn-1, River Drake)** have a source-matched partial silhouette/matte repair reviewed on red, white,
  gray and black, with hash-bound reviewed masks and any edge correction evidence.
* `authored-alpha-preserved`: **1 plate**, Vampire's original RGBA pixels are preserved exactly;
  its pre-existing partial alpha is **not** represented as newly repaired.
* `legacy-reference-needs-manual-mask`: **77 plates**; uncertain or rejected candidates do **not**
  ship. The old native WebP decode + historical cutout is frozen as RGBA, byte-exact
  in decoded pixels. This preserves existing presentation, **not a claim of a fix**.

The automatic Horn-6 proposal was explicitly rejected for deleting dark chest,
shoulder/foreleg and leg-joint anatomy. All Zillennia stages remain in the manual
queue: their black bodies and thin purple tendrils are intentional. Neither a
low grayscale value nor a numerical confidence score authorizes their removal.

**Fully cleaned/fully resolved plates: 0 claimed.** All 80 originally opaque plates still have manual work (77 full silhouette reviews and 3 residual edge reviews). Phoenix locally corrects 1,895 retained edge pixels and recovers 1,264 weak-rim samples; 6,296 more ambiguous source pixels are preserved verbatim rather than discarded. Every source sample with peak RGB above 32 retains support, including the specifically disputed flame pixels. This is a conservative support guard, **not semantic segmentation confidence** or proof that every lower-signal dark detail is correctly classified. Residual rims/pockets are expressly not called fixed.

## Reproduce / review

Install `tools/creature-alpha/requirements.txt` in a workspace virtual environment.
The maintainer dependencies are not required by the game. OpenCV RNG seed is zero,
threads one, graph-cut iterations five, with dependency versions in each report.

```
python tools/creature-alpha/pipeline.py audit
python -m unittest discover -s tools/creature-alpha/tests -v
python tools/creature-alpha/test_edge_matte.py
python tools/creature-alpha/pipeline.py build
python tools/creature-alpha/pipeline.py verify
sh tools/cargo.sh test -p corpse-desktop embedded_assets
sh tools/cargo.sh test -p corpse-desktop manual::tests
```

`audit --only phoenix horn-1` restricts work. Auditing writes ONLY under
`art-review`: original-vs-legacy-vs-candidate comparisons, full-resolution four
background composites, mask-difference maps, initial graph-cut labels, per-asset
risk indicators and a manual queue. It cannot overwrite masters or promote art.
All 81 plates were audited. Lack of a review is not acceptance.

For opaque masters the perimeter model proposes exterior seeds only. OpenCV
GrabCut fits foreground/background color mixtures and performs a spatial graph
cut over the seeded image graph, including its unknown region. Hard foreground protects strong disconnected details
and (by default) dark interior cores. No largest-component pruning is done.
Distance transforms select seeds; they never erode, close, blur or feather the
output silhouette. This remains fallible: color similarity is not semantic proof.
Reviewed foreground/background trimaps and specific negative-space pocket seeds
can override uncertain proposals. Seed reviews are bound to the original SHA.

Phoenix uses an asset-specific reviewed graph-cut silhouette with the general
interior guard disabled; this decision must not be copied to dark-bodied assets.
For eligible edge samples, the separate local matte module solves the original
black-matte compositing equation against stable local opaque color references.
It does not average output colors or alpha, never deletes reviewed silhouette
support, and leaves unsupported cases untouched/flagged. Fractional alpha, when
accepted, is recovered source coverage, not a feathered binary edge. Changes
must reconstruct source RGB on its old matte within two byte levels and be
confined to a reviewed one-pixel boundary/support mask. This is conditional
reconstruction, not recovery of an unknowable original alpha with certainty.

Promotion requires explicit review decisions in `tools/creature-alpha/reviews.json`:
original and approved RGBA SHA, reviewed alpha mask SHA, named reviewer, notes,
eight review checks, and (when used) correction/binary-mask hashes. Authoritative
approved PNGs and masks live in `tools/creature-alpha/reviewed`, separate from
candidates. `build` rejects stale source/mask hashes, resized art, arbitrary RGB
changes, unapproved fractional alpha, or invalid matte reconstruction. No model
score can substitute for the review gate. Manually inspect flames/tendrils,
small disconnected features, outlines, internal black details and source overlays.

`tools/creature-alpha/legacy-provenance.json` binds old decoded RGBA hashes to the
historical native algorithm. The frozen runtime PNG doubles as the unreviewed
reference, avoiding a duplicate authoritative tree. If a reference is missing,
regenerate in an isolated directory, **not over reviewed runtime images**:

```
sh tools/cargo.sh run -p corpse-desktop --example freeze_legacy_alpha -- \
  crates/corpse-desktop/assets/creatures-distribution art-review/legacy-reference
python tools/creature-alpha/pipeline.py build
```

The historical lossy WebP files and `art-distribution.json` are retained for this
reference only; the game does not read them. `tools/prepare-native-art.py` now
invokes the review-gated RGBA builder instead of re-encoding opaque runtime WebPs.
The source build is self-contained without the large optional `art-review` tree. The shipped audit-summary.json and manual-queue.json retain inventory measurements and unresolved work even when that visual review tree is omitted.

## Complexity and bounds

Let N=81, P=width×height for one plate, E≤8P neighboring graph arcs, K=5 mixture
components and I=5 graph-cut iterations. Offline image decode/encode, distance
transforms, source/mask scans and compositing are O(P) time/extra memory. Perimeter
quantiles in the seed proposal sort O((W+H) log(W+H)) values. Model fitting is
O(IKP); graph max-flow uses the library implementation and has an unverified conservative
polynomial planning estimate O(I×P²×E), not a solver-specific proven bound or a claim of O(P) segmentation. Practical
timings depend on graph structure and must be measured. Storage is O(P+E+K) per
plate, plus encoded candidates/review sheets O(NP) on disk. Review pocket count
is explicitly bounded by the input list; each proposal currently scans O(P).
The bounded-radius local edge estimator examines a fixed-size neighborhood per
eligible pixel, O(P) time/memory; no semantic confidence is implied by this bound.

Runtime cold selection: O(encoded bytes + P) PNG decompression, O(P) trim scan/copy,
and O(P) decoder/output/cache storage. Warm frames have no segmentation, image
file reads or decode. Native drawing still costs O(destination coverage), with
nearest sampling. Only one selected plate is cached; during switches old/new
pictures can coexist. All original per-image dimensions remain unchanged.
