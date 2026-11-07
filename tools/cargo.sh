#!/bin/sh
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
export CARGO_HOME="$ROOT/.cargo"
export RUSTUP_HOME="$ROOT/.rustup"
export PATH="$CARGO_HOME/bin:$PATH"
exec cargo "$@"
