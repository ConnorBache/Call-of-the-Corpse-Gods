> **Small-package follow-up:** The quality-100 28,354,828-byte ZIP exceeded the native attachment limit. Current distribution uses approved quality 90 for creature RGB (alpha and dimensions exact) plus strictly lossless WebP for four UI resources. No source PNGs or gameplay changed. Earlier q100 verification is retained under `verification/r39-compact`; the new input freeze/gate is `verification/r39-compact-small`.

# Compact r39 runtime artwork

This packaging-only update keeps simulation revision 39 and changes no gameplay, authored text, sprite dimensions, silhouette, alpha, or drawing algorithm. The designer approved high-quality color compression after lossless WebP still occupied 52.97 MiB for art alone. Color compression can change colors and fine detail; this is not lossless or pixel-identical RGB.

The 84 distribution plates in `crates/corpse-desktop/assets/creatures-runtime` are derived from the current `creatures-rgba` PNGs, not the historical lossy assets. All authored masters, segmentation/manual-mask inputs, and true-RGBA PNG exports remain unchanged. `creature-runtime-manifest.json` records selected quality, encoder versions, source/output hashes, exact alpha checks and visible-RGB error measurements. `tools/prepare-compact-art.py` reproduces the derivation and rejects changed dimensions or any changed alpha byte. No resizing, smoothing, dark-pixel removal, color-keying, or new segmentation occurs.

Runtime decoding uses the already pinned `image-webp = 0.2.4`. Only selected manual plates are decoded and cached. The original integer-nearest sampling and alpha compositing remain unchanged. Native tests compare all 84 decoded alpha planes and dimensions to their canonical PNGs. Independent verification additionally uses Pillow and FFmpeg and four-background comparison sheets. Color changes are assessed visually, not claimed absent.

## Complexity and bounds

Let N be plate count (84), L maximum ID length, B total encoded bytes and P selected plate pixels. Build indexing takes O(N log N × L) comparisons, O(NL) index storage, and O(B) resource validation/I/O. Canonical PNGs are checked but not embedded. Lookup is O(log N × L) with O(1) extra memory. A cold selection decodes in O(encoded bytes + P) time under the fixed codec, with O(P) scratch/output memory; a warm selection performs no decode. Dimensions are rejected outside 1..4096 in either axis and animation is rejected before output allocation. Selected RGBA requires 4P bytes; current 1254² plates require about 6 MiB. Switching may transiently retain both old and new plates. The whole catalog is not eagerly decoded.

Offline encoding and quality metrics visit all pixels and allocate O(P) working arrays per parallel worker. WebP method 6 uses fixed codec search settings; no asymptotic speed claim is made for its internal compression search. Measured encode times are recorded separately. Exact alpha validation is O(sum P) across the catalog. Native and independent checks remain necessary after changing an encoder or decoder.

## Packaging

`tools/package-compact-windows.py` produces a conventional ZIP with executable, SDL2 runtime DLL, required notices and a short readme. Symbol stripping is performed on a packaging copy and checked to preserve PE section contents. No packer, installer, self-extractor, network asset download, or runtime filter is used. The Windows binary is cross-compiled and unsigned; build/tests and PE inspection are not a claim of Windows launch/input/save validation.

## Smaller-package build and UI resources

`tools/prepare-compact-ui.py` derives lossless, exact-RGBA WebP representations of menu logo, broad ink splash, Brogue atlas and window icon into `assets/ui-runtime`. Every byte, including hidden RGB under zero alpha, must decode exactly. `assets/ui-runtime-manifest.json` records hashes. The corresponding loaders use the same bounded decoder; sampling/compositing and SDL icon setup are unchanged. The PNG dependency remains for screenshot/render-test encoding and canonical-image tests. No command-line functionality was removed.

The small release uses these standard Cargo profile overrides (recorded with each packaged binary):

```sh
CARGO_PROFILE_RELEASE_OPT_LEVEL=s \
CARGO_PROFILE_RELEASE_LTO=fat \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_DEBUG=0 \
CARGO_PROFILE_RELEASE_STRIP=symbols \
cargo build --release --locked --jobs 2 --target x86_64-pc-windows-gnu -p corpse-desktop
```

Cross-SDK/linker setup remains as documented in `windows-cross-build.md`. These flags optimize compiled code size, not simulation rules or data; performance measurements and limitations are retained with the small-release verification. `tools/package-compact-windows.py --label r39-compact-small --build-profile-json <recorded-profile.json> ...` retains the profile and both artwork manifests.

UI decoding cost is O(encoded bytes + UI pixels) with O(UI pixels) output/scratch memory, once at relevant initialization/cache miss, under the same 4096-per-axis bound. Four additional decoded canonical equality checks are O(sum UI pixels). No new per-frame decode is introduced.

### Measured profile choice

The opt-s candidate is 19,599,802 bytes. A single sequential Linux comparison on the shared cloud host found median per-seed command-time ratio 1.176× versus the previous opt3/thin-LTO profile (range 1.155–1.338×): roughly 17.6% slower in those six initial-floor traces. Maximum sampled per-seed command median was 11.38 ms versus 8.50 ms. Warm-page mean ratios had median 1.024×; actual means were 2.48–7.74 ms versus 2.50–7.28 ms. This is a real measured performance tradeoff, not a claim of unchanged performance. Opt-z was rejected: median core ratio 1.615× and warm-page ratio 2.077×. All six release trace runs retained canonical simulation parity. These bounded samples are not Windows FPS or deep-floor worst-case claims; raw results are retained in `verification/r39-compact-small/performance`.
