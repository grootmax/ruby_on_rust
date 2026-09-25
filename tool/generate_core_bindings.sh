#!/bin/sh
set -e

JIT_SRC_ROOT_PATH="${JIT_SRC_ROOT_PATH:-.}"
export JIT_SRC_ROOT_PATH

if [ $# -eq 0 ]; then
  EXTRA_FLAGS=""
  if [ -d "${JIT_SRC_ROOT_PATH}/.ext/include" ]; then
    for d in "${JIT_SRC_ROOT_PATH}"/.ext/include/*; do
      if [ -d "$d" ]; then
        EXTRA_FLAGS="$EXTRA_FLAGS -I$d"
      fi
    done
  fi
  exec cargo run --manifest-path "${JIT_SRC_ROOT_PATH}/jit/bindgen/Cargo.toml" -- -I"${JIT_SRC_ROOT_PATH}" -I"${JIT_SRC_ROOT_PATH}/include" -I"${JIT_SRC_ROOT_PATH}/.ext/include" $EXTRA_FLAGS
else
  exec cargo run --manifest-path "${JIT_SRC_ROOT_PATH}/jit/bindgen/Cargo.toml" -- "$@"
fi
