#!/bin/sh
# Builds the macpurge CLI and stages it as a Tauri resource so the app can install it.
set -e
cd "$(dirname "$0")/.."
cargo build --release --manifest-path ../cli/Cargo.toml
mkdir -p src-tauri/resources
cp ../cli/target/release/macpurge src-tauri/resources/macpurge
