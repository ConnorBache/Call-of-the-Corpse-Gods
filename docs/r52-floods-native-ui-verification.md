# R52 Floods native UI verification

The current release split restores this targeted-wave suite with a separate
Pack target-mode regression. The receipts below remain historical; see the
[current release scope](revision52-released-floods.md).

**Source reconstruction note, 2026-11-08:** The receipts and exact source hash
below describe the historical release. Its missing test source has since been
functionally reconstructed, not recovered byte-for-byte. The replacement run
has six passing tests, 58 PNGs and 116 exact framebuffer comparisons. See
[the reconstruction record](revision52-desktop-source-restoration.md).

## Scope

`crates/corpse-desktop/src/revision52_floods_ui_tests.rs` is a dedicated test-only module. Its only registration is one `#[cfg(test)]` module entry in `main.rs`. It does not change gameplay, renderer behavior, palette, targeting, damage tracking, or the frozen all-role balance catalog.

The authored scene is an admitted 23×23 room with neutral 100 ambient illumination, a real tier-one Floods amulet, and the default appearance profile. Public `engine::dispatch` and `PackTarget` paths produce the terrain, targets, movement, receipts and normal-turn expiry. Each captured view comes from the ordinary observation pipeline. Full, retained Scene/base and tile Painter pathways render the same detached view at 1280×800 and 960×1200. No images are manually painted, edited or composited.

Six focused tests cover:

- First cast leaves 15 shallow cells. The next cast consumes the first shallow cell in each lane into three deep cells. A third cast cannot propagate through those deep cells.
- Normal-turn `Wait` expiry restores original floor, with refreshed deep cells lasting beyond the original shallows. Natural shallow water returns to shallow after deepening expires.
- Mixed shallow, deep and open-floor lanes stop independently. Mouse and keyboard geometry match the actual A12 travel receipt on visible terrain.
- Hidden live shallow/deep/floor mutations do not change known-only aiming or target admission.
- Actual A12 wave receipt coordinates drive the native animation. Full, retained and tiled pixels match at each sampled animation frame and after removal.
- Real movement into deepened water activates the existing immersion tint; moving out clears it. R51 still stops before shallow water without deepening it.

Every raster comparison is exact RGBA byte equality, including aiming dismissal and expired effects. No tolerance, changed-pixel exclusion or forced full-frame shortcut is used. Captures show actual CPU-native pixels; they do not prove Windows runtime behavior, accelerated GPU output, SDL input delivery or physical display DPI.

## Verification evidence

Final dedicated run: `verification/r52-floods-ui-03/`.

- `tests.log` and `tests.exit`: 6 passed, 0 failed, exit 0. The filtered integration targets ran zero selected cases and are not counted as additional passes.
- `source-before.json` and `source-after.json`: identical complete core/desktop Rust and JSON source manifests, including manifests, lockfile, toolchain, room input and Cargo wrapper.
- `captures/`: 50 actual PNG frames, 25 each at 1280×800 and 960×1200. `image-hashes.json` records every image's SHA-256 and size. The harness performs 100 exact full-versus-retained/tiled framebuffer comparisons over those 50 frames.
- `started.txt` and `finished.txt`: execution timestamps.

Related run: `verification/r52-floods-ui-related/`, also with identical before/after source manifests and all four exit codes zero. R51 aiming: 4 passed. R52 aiming: 2 passed. Water-depth rendering: 9 passed, 1 intentionally ignored isolated native-GPU test. R52 amulet UI: 8 passed. Total related results: 23 passed, 1 ignored. These are focused checks, not the full workspace or Windows build.

Representative final captures were visually inspected at both dimensions: first-cast light-blue shallows, second-cast rich-blue consumed edge, natural-water mixed lanes before/after, A12 animation and cleared aiming. The immersed-player frame was also inspected; the existing blue tint preserves readable map and HUD. No renderer defect or runtime change was needed.

Earlier failed receipts are preserved. `r52-floods-ui-01` used an invalid rectangular test map; native geometry admits square arrays or the canonical 79×29 shape. The fixture was corrected to 23×23 and now explicitly passes public validation before use. `r52-floods-ui-02` passed all five R52 tests but incorrectly expected an R52 animation receipt from R51; that legacy-only assertion now respects the existing receipt revision gate. Neither correction weakened runtime validation or the exact pixel oracle.

The frozen all-role balance catalog remains SHA-256 `15c968d8361c7198ead29c76f763997cdbdae77b40af616bf572a0dc4c5c725f`. The final dedicated test module is SHA-256 `b3a3def7af391268a62662e0d711ff93edaf69a02873c82fa784c5e245aa435d`.

## Test harness complexity

Let F be sampled frames, P native framebuffer pixels, N floor cells, C observed paint layers, E emitted effect tiles, and T temporary-terrain records. The exact byte oracle adds O(F·P) time and O(P) memory beyond production observation/rendering. Three Painter buffers, one retained base buffer and bounded damage trackers are retained per size. Snapshot views add O(F·C) test memory. PNG encoding costs O(P) memory per image plus encoder work; it is verification overhead, not runtime work.

Production observation, Scene capture, tile planning, text rasterization and effect projection retain their documented costs and caches. The harness does not claim they are O(P) or omit them from measured test duration. It does not benchmark runtime speed. The authored footprint is at most 15 cells, the room has 529 cells, and two fixed image sizes bound raster work. Normal-turn expiry is exercised by public commands, including their ordinary engine work, rather than by a test-only direct terrain edit.
