#!/bin/sh
# Builds the macpurge CLI and stages it as a Tauri resource so the app can install it.
# Honors TAURI_ENV_TARGET_TRIPLE (set by `tauri build --target ...`) for cross builds.
set -e
cd "$(dirname "$0")/.."
T="${TAURI_ENV_TARGET_TRIPLE:-}"
if [ -n "$T" ]; then
  cargo build --release --manifest-path ../cli/Cargo.toml --target "$T"
  BIN="../cli/target/$T/release/macpurge"
else
  cargo build --release --manifest-path ../cli/Cargo.toml
  BIN="../cli/target/release/macpurge"
fi
mkdir -p src-tauri/resources
cp "$BIN" src-tauri/resources/macpurge
