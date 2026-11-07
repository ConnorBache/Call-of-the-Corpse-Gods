#!/bin/sh
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
SDK="$ROOT/native-windows-sdk/sysroot"
export PATH="$SDK/usr/bin:$PATH"
SDL_LIB="${SDL2_LIB_DIR:-$ROOT/native-windows-sdk/SDL2-2.32.10/x86_64-w64-mingw32/lib}"
exec "$SDK/usr/bin/x86_64-w64-mingw32-gcc-posix" -B"$SDK/usr/lib/gcc/x86_64-w64-mingw32/14-posix/" -B"$SDK/usr/x86_64-w64-mingw32/lib/" -L"$SDK/usr/x86_64-w64-mingw32/lib" -L"$SDL_LIB" "$@"
