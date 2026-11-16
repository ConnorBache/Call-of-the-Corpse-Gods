# R52 desktop source reconstruction

This is a functional reconstruction from the saved R52 design documents, recovered callers and public engine APIs. It is **not byte-exact recovery**. None of the three reconstructed file hashes matches the missing release source manifest. `manifest.json` records the exact base, reconstructed and expected hashes and sizes. `desktop-source-restoration.patch` modifies only the three owned desktop files and dry-runs against the untouched candidate.

## Runtime restoration

- `stat_presentation.rs`: adds the missing `enemy_damage_revision` used by the recovered Bestiary. The nominal species strike uses its own `weaponDamage` and `weaponOffenseMultiplier` with its raw Offense, followed by the existing attack scale of 0.5. R51+ uses the production nearest-damage helper; earlier revisions use the production legacy ceiling helper. Negative damage remains clamped to zero. Current `enemy_damage`, bonus conversion and signed player HUD rounding remain available. No compile-only shim is used.
- `main.rs`: routes only the originating Game or ReplayGame run's simulation revision into the recovered Bestiary setter. Missing legacy revisions remain revision 29. Menu/inactive/absent runs retain the current revision 52 default. The helper reads no actors, terrain, RNG or hidden state. Existing event/inspect wiring is preserved. Registers the Floods test module under `cfg(test)`.
- `revision52_floods_ui_tests.rs`: reconstructs the six documented behavioral/CPU-raster tests using a publicly validated square 23×23 fixture, public dispatch, real PackTarget, ordinary observation and unmodified production renderer.

## Deliberately separate pending features

The main inspection-footer and responsive ability-detail feature patch is not absorbed here. `inspect-dialog-work/main-integration.patch` dry-runs cleanly after this restoration (all hunks offset by 41 lines). Apply that patch centrally with its renderer counterpart. Existing candidate changes in other desktop files were retained in the isolated build but are not re-delivered in this patch. Other newly requested balance, glyph, animation, Bestiary and entity-list changes remain the integration owner's responsibility.

## Verification

The reconstruction verification artifacts accompany this source delivery:

- Six reconstructed Floods tests: 6 passed, 0 failed. 58 native PNGs at 1280×800 and 960×1200, with 116 exact full-versus-retained/tile RGBA comparisons. No tolerances, ignored pixels or forced-full flags were used. Initial fixture failures (stale logKinds, and an accidentally over-perceptive hidden-terrain fixture) are retained in separate logs and were fixed only in test setup.
- Source manifests before/after the final Floods run are identical, including Rust/JSON sources, Cargo manifests/lockfile/toolchain, fixture and Cargo wrapper.
- Related suites: revision routing 1 passed; stat presentation 4 passed; aiming 6 passed; water-depth 9 passed and 1 existing isolated GPU test ignored; R52 amulet UI 8 passed.
- Representative landscape/portrait captures were inspected: first shallows, second-cast deep edge, mixed aiming, real wave animation, aiming dismissal and immersion tint. No image was edited or composited.
- All three core runtime files used for the final desktop run match the core restoration worker's final runtime files. The core test module in this staging copy predates that worker's final test-only strengthening; this does not change the production core used by the desktop tests.

These checks do not prove the original missing bytes, Windows runtime behavior, GPU output, SDL input delivery, physical DPI or final merged workspace correctness. A final locked workspace gate and Windows cross-build remain the release owner's responsibility. The old 50-image R52 release report describes the historical missing test source, not this reconstruction's 58-image run.

## Complexity and known uncertainty

The restored revision handoff and stat arithmetic use O(1) time and O(1) extra memory per call. Existing Bestiary per-floor birth/catalog work and font measurement/rasterization retain the bounds already in `docs/revision52-bestiary-stats.md`; no new cache or gameplay work is introduced.

Let F be sampled frames, P framebuffer pixels, C observed layers, E emitted effect tiles and N floor cells. The replacement UI byte oracle adds O(F·P) comparison time and O(P) buffers, plus O(F·C) stored detached views, outside production observation/Scene/tile/effect/glyph costs. PNG encoding adds O(P) memory per encoded frame. Its six scenes have N=529, at most 15 reached Floods cells and fixed two-size raster bounds. Public Wait/Move/cast commands retain their real engine costs. These are source-derived bounds, not performance measurements. Existing linked Floods verification complexity documentation still applies, but the current capture count is 58.

Exact original names, formatting, helper shape, test fixtures/assertions and original registration location are unknowable from the available source fragments. The reconstructed APIs and runtime decisions follow the recovered consumers and explicit documents, not guessed byte-padding. Original tests may have had additional assertions absent here. No release-binary differential execution or decompilation was performed.
