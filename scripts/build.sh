#!/usr/bin/env bash
# Build a release of Sonora.
#
# Run inside the dev shell:  nix develop --command bash scripts/build.sh
#
# Produces:
#   - standalone binary:  src-tauri/target/release/sonora
#   - bundles (Linux):    src-tauri/target/release/bundle/{deb,appimage}/...
#
# Pass extra args straight to `tauri build`, e.g. to pick bundle targets:
#   scripts/build.sh --bundles deb
set -euo pipefail

cd "$(dirname "$0")/.."

echo "› installing frontend deps…"
bun install

echo "› building release (this is slow: LTO + whisper.cpp + webkit)…"
bun run tauri build "$@"

echo
echo "Done."
echo "  binary : src-tauri/target/release/sonora"
echo "  bundles: src-tauri/target/release/bundle/"
