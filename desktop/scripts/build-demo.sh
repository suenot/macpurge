#!/bin/sh
# Build the real desktop UI against the in-memory mock backend and copy it into
# the landing site (served at /demo/). Usage: scripts/build-demo.sh [landing-dir]
set -e
cd "$(dirname "$0")/.."
LANDING="${1:-../../macpurge-landing}"
NEXT_PUBLIC_DEMO=1 npx next build
rm -rf "$LANDING/public/demo"
cp -R out "$LANDING/public/demo"
echo "demo copied to $LANDING/public/demo"
