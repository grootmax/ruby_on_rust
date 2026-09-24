#!/bin/sh
set -e

# Default JIT_SRC_ROOT_PATH to repository root if not set
if [ -z "$JIT_SRC_ROOT_PATH" ]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    JIT_SRC_ROOT_PATH="$(cd "$SCRIPT_DIR/.." && pwd)"
    export JIT_SRC_ROOT_PATH
fi

CARGO="${CARGO:-cargo}"

exec "$CARGO" run --manifest-path "$JIT_SRC_ROOT_PATH/jit/bindgen/Cargo.toml" -- "$@"
