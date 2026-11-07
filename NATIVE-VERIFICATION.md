# R51 verification status

The final unfiltered cloud Linux workspace run passed **202 targets, 2,296 test
executions, zero failures, and 42 ignored executions**. Ignored checks did not run
in that aggregate. Rust 1.99.1, optimized tests, debug assertions and overflow
checks were enabled. The actual command, completed log and exit code are in
`verification/r51-ui/test-report.json`.

Windows x64 release cross-compilation passed. The 113-member player ZIP was
read back byte-for-byte, with pinned SDL, imports, artwork and notices checked.
Independent PE validation verifies retained code/data, selected invariant headers,
aligned file layout, and the exact embedded DPI manifest after debug stripping.
Windows runtime, physical GPU and monitor/DPI transitions remain **unverified**.
No user computer was accessed.

Runtime/test inputs match source snapshot `7dfe86d997d3bcca56b381b9ef77ec335fdcc8baa39bf83a5631e04a6880e7be`.
A metadata-only packaging receipt correction occurred during the Rust gate;
all Rust inputs stayed byte-identical, its 9 Python tests passed, and both
snapshots plus the exact delta are retained. This is not a claim that every
source file stayed unchanged throughout the run.

The illumination calibration matches all nine reference colors, with 55 native
scenes identical across full/retained/tile rendering. Linux offscreen OpenGL
maximum channel difference was 2; water comparison maximum was 3.

See [release scope and open decisions](docs/revision51-release-status.md),
[the follow-up index](docs/revision51-follow-up-index.md), and
`verification/r51-ui/final-verification.json` for precise evidence and limits.
These checks do not close unresolved design decisions or reproduce the user's
original floor-40 unsupported-behavior notice.

## Historical incremental-save verification

See [the engineering report](docs/verification/incremental-save-engineering-report.md) and `verification/incremental-save/`. The assertion-enabled optimized native gate passed 164 targets: 1306 passed, 0 failed, 13 ignored test executions (not unique tests). Older v18 receipts are preserved unchanged under `verification/historical-pre-incremental-save/`. Windows cross-compilation and static archive verification are not Windows runtime verification. Process-kill tests are not physical power-loss certification.
