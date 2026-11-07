#!/bin/sh
# Build-local executable dispatcher; restored source script remains byte/mode unchanged.
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec /bin/sh "$ROOT/source/tools/windows-linker.sh" "$@"
