# Workspace-only Windows cross-build dependencies

Prepared 2026-11-06 UTC for `/workspace/scratch/69f53637ba6f/r50-work`.

## Scope and validation

- Nothing was installed into system directories or onto the user's computer. Debian archives were extracted with `dpkg-deb --extract`; no package manager or package maintainer scripts were run.
- Six Debian MinGW packages were downloaded from the exact `https://deb.debian.org/debian/pool/` URLs pinned by the repository's `verification/integration-r39/windows-build/debian-package-manifest.json`. Every archive matched its recorded SHA-256 and byte length before extraction. Per-archive JSON receipts and HTTP response headers are in this directory. Archive control metadata is in `debian-control-metadata.json`. This check is not a new independent Debian signed-index verification.
- SDL2 2.32.10 was downloaded from the pinned official `libsdl-org/SDL` GitHub release. Its bytes/hash match the repository manifest. Its official GitHub release asset URL, byte size and published digest were independently compared with the pinned values. See `sdl-official-asset-verification.json` and `sdl-official-release.json`.
- Official rustup installed the `x86_64-pc-windows-gnu` standard library into the already-installed workspace Rust 1.99.1 toolchain. Rustup validates its download checksum. The target's hash is recorded in the installed toolchain manifest and `rust-target-manifest.json`; the separately fetched official SHA-256 sidecar matches it. See `rust-target-published.sha256`.
- `rustup target list --installed`, `rustc -Vv`, GCC/cpp/ld/windres version probes, SDL version-header inspection and `ldd` dependency resolution completed successfully. See `toolchain-validation.txt`.
- No game builds, Cargo tests, release-gate edits, source changes or Windows execution were performed for this preparation task. No Wine/Windows runtime is installed. Cross-build success does not establish launch, input, rendering, save or gameplay behavior on Windows.

## Installed paths

- Cargo and rustup: `/workspace/scratch/69f53637ba6f/.cargo/bin/`
- Rust toolchain: `/workspace/scratch/69f53637ba6f/.rustup/toolchains/1.99.1-x86_64-unknown-linux-gnu/`
- Windows Rust target libraries: `lib/rustlib/x86_64-pc-windows-gnu/` inside that toolchain.
- SDK: `/workspace/scratch/69f53637ba6f/native-windows-sdk/`
- MinGW programs: `sysroot/usr/bin/x86_64-w64-mingw32-*` inside the SDK. GCC/cpp are the `-posix` variants.
- SDL libraries: `SDL2-2.32.10/x86_64-w64-mingw32/lib/` inside the SDK.
- SDL runtime DLL: `SDL2-2.32.10/x86_64-w64-mingw32/bin/SDL2.dll` inside the SDK.

## Reproduce installation

Run from the repository directory; the preparation script retains the exact manifest-based download, hash/size checks and extraction procedure:

```sh
python3 ../native-windows-sdk/prepare-pinned-sdk.py
CARGO_HOME="$PWD/../.cargo" RUSTUP_HOME="$PWD/../.rustup" \
  ../.cargo/bin/rustup target add --toolchain 1.99.1 x86_64-pc-windows-gnu
```

The SDK script consumes the repository's pinned manifests and rechecks cached archives before extracting. The initial bootstrap created `downloads`, `receipts` and `sysroot` under the SDK directory.

## Build command for the parent task, not run here

```sh
SDK="$(cd .. && pwd)/native-windows-sdk"
PATH="$SDK/sysroot/usr/bin:$PATH" \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$PWD/tools/windows-linker.sh" \
SDL2_LIB_DIR="$SDK/SDL2-2.32.10/x86_64-w64-mingw32/lib" \
sh tools/cargo.sh build --release --target x86_64-pc-windows-gnu -p corpse-desktop
```

The existing `tools/windows-linker.sh` is executable. The build resource script already explicitly selects `x86_64-w64-mingw32-cpp-posix`, so no global alternatives or new generic compiler symlinks were needed. Before distribution inspect actual executable DLL imports, include only required runtime DLLs/notices, and verify on Windows. Do not ship the SDK/compiler with the player download.
