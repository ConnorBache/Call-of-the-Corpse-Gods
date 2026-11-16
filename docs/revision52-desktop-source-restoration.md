# R52 desktop source reconstruction

Three missing/outdated desktop source files were functionally reconstructed on
2026-11-08 from the saved R52 documents, recovered callers and public engine
APIs: `main.rs`, `stat_presentation.rs`, and `revision52_floods_ui_tests.rs`.
None matches the missing original source hash. The reconstructed implementation
is not a byte-exact recovery, and prior release receipts remain historical.

`enemy_damage_revision` restores actor-specific nominal weapon plus Offense
scaling at 0.5, using legacy ceiling before R51 and nearest rounding from R51.
The main handoff sends only the originating Game/ReplayGame revision to the
Bestiary; absent/inactive/menu runs default to R52, missing legacy revisions to
R29. No simulation, actor, RNG, saved state or discovery state is changed.

The six replacement Floods tests exercise real casts, mixed lane preview and
receipts, repeated shallow-to-deep conversion, normal-turn expiry, remembered
hidden terrain, A12 animation removal, movement immersion and R51 stopping.
They pass with 58 native PNGs (1280x800 and 960x1200), and 116 exact comparisons
between full, retained and tiled RGBA buffers. Representative frames were
visually reviewed. The final run's 658-record source manifests are identical.
Related routing, stat, aiming, water and amulet checks also pass. One existing
isolated water GPU test is ignored. No full-frame force flag, tolerance or
changed-pixel exclusion is used. No image is painted or composited manually.

The restored scalar routing and arithmetic are O(1) time and extra memory.
Bestiary birth/catalog reconstruction and font work retain the bounds in
revision52-bestiary-stats.md. Let F be frames, P native pixels, C observed layers,
E emitted effect tiles and N floor cells. The CPU oracle adds O(F*P) comparison
time/O(P) buffer memory and O(F*C) detached views, beyond production observation,
Scene/tile planning, O(N+C+E) effect projection and native glyph raster work.
PNG encoding adds O(P) memory per frame. N=529, reached wave cells <=15, and
two fixed raster sizes bound these test scenes. Public commands retain their
actual engine costs. These are source-derived bounds, not speed measurements.

The replacement test's precise assertions/fixtures differ from the missing
original. Original formatting/helper names/registration location are unknown;
there may have been other original assertions. Separate pending inspection and
other UI features are integrated independently. Native CPU evidence does not
prove Windows runtime, accelerated GPU, SDL input delivery or physical DPI.
The release owner must retain reconstruction manifests and receipts alongside
the final locked workspace and Windows cross-build gates.
