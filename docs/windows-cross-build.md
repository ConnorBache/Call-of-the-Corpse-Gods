# Windows native cross-build procedure

The build host is Linux. Cross-compiling an executable does **not** verify Windows launch, input, rendering or save behavior. Keep this distinction in release notes.

Toolchain dependencies are unpacked into the parent workspace's `native-windows-sdk`, without administrator installation or package maintainer scripts:

- Official Debian Trixie MinGW-w64 compiler, binutils, runtime and headers. Downloads are checked against each package's published SHA-256; `debian-package-manifest.json` records exact URLs and hashes.
- SDL2 2.32.10 development archive from the official libsdl-org/SDL GitHub release. `sdl-manifest.json` records the artifact URL and hash.
- Official rustup target `x86_64-pc-windows-gnu` for the pinned Rust 1.99.1 toolchain.

`tools/windows-linker.sh` points the linker at the unpacked workspace tools. Build from this native repository:

```sh
SDK="$(cd .. && pwd)/native-windows-sdk"
PATH="$SDK/sysroot/usr/bin:$PATH" \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$PWD/tools/windows-linker.sh" \
SDL2_LIB_DIR="$SDK/SDL2-2.32.10/x86_64-w64-mingw32/lib" \
tools/cargo.sh build --release --target x86_64-pc-windows-gnu -p corpse-desktop
```

Before packaging, inspect the executable's DLL imports, include required redistributable DLLs and notices, preserve the relative runtime asset directory structure, and test on Windows. Never bundle a compiler/sysroot with the player download. Repository licensing remains the designer's decision; dependency notices do not grant a license for the game itself.

The linker helper explicitly forwards `SDL2_LIB_DIR` as a native library search path (defaulting to the bundled SDL 2.32.10 SDK). A cached sdl2-sys build does not necessarily consume that environment variable itself; forwarding it avoids a missing `-lSDL2` failure without changing the DLL or rebuilding dependency code.
