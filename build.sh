#!/bin/bash

declare CANISTER_NAME=$1
declare TARGET_DIR=./target/wasm32-unknown-unknown/release

# build
if ! cargo build --release --target wasm32-wasip1 --package ${CANISTER_NAME}; then
  echo "Build failed." >&2
  exit 1
fi

# wasi2ic
if [ ! -d ${TARGET_DIR} ]; then
  mkdir -p ${TARGET_DIR}
fi
if ! wasi2ic ./target/wasm32-wasip1/release/${CANISTER_NAME}.wasm ${TARGET_DIR}/${CANISTER_NAME}.wasm; then
  echo "wasi2ic failed." >&2
  exit 1
fi

# candid-extractor
if ! candid-extractor ${TARGET_DIR}/${CANISTER_NAME}.wasm > src/${CANISTER_NAME}.did; then
  echo "candid-extractor failed." >&2
  exit 1
fi
