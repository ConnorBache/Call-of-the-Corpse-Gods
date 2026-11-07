# R51 verified cloud candidate

The implemented scope is indexed in [the follow-up index](revision51-follow-up-index.md).
The final unfiltered Rust workspace passed 202 targets: 2,296 test executions
passed, zero failed, 42 ignored. Windows x64 cross-compilation, PE/DLL/import
checks and full player-ZIP read-back passed. This is not Windows execution.

## Decisions still open

- Speed-up destination labels for ordered connections with multiple destinations need the designer's final format.
- Iceblast delivery into a blocker: detonate there versus reject remains undecided. Existing area-target behavior is preserved.
- Ordinary Sparkle fire spreading into Ichor-covered water remains undecided. Direct wet casting already quenches; independent Ichor combustion and Hellfire are preserved.

The original floor-40 unsupported-behavior error remains unisolated. The separate
DEBUG stale-root checkpoint divergence is fixed and its save/resume regression
passes. Do not describe that as proof of the original report's cause.

## Verification limits

- Cloud Linux only; no user PC accessed.
- Windows cross-compilation and static archive/PE validation are not Windows runtime, physical GPU, monitor-switch or DPI transition verification.
- The full Rust gate has 42 ignored test executions. Additional targeted offscreen SDL/OpenGL checks are reported separately; ignored tests are not claimed as executed.
- The original floor-40 unsupported-behavior notice has not been reproduced. A separately identified DEBUG stale-root save/resume divergence is fixed and regression-tested.
- The source checkpoint depends on the separately identified canonical19 baseline; it is not a standalone copy of all unchanged assets.
- Default calibrated illumination follows the specified curve; legacy blending/maximum-illumination editor controls do not affect the calibrated display path.
- Deterministic archive ordering does not establish byte-reproducible executables: the strip tool may update PE timestamps and checksums.

The player archive is revision51-windows-x64.zip. Extract the entire archive and
keep SDL2.dll beside the executable. It is unsigned; do not bypass a security
warning merely because cross-build/static checks passed. Source, test and build
reports are matched to the final snapshot, including the documented packaging
metadata-only delta. No claim of GitHub publication is made here.
