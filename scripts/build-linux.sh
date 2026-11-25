#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> 1. Running workspace release tests & latency-contract benchmarks"
cargo test --workspace --release
cargo run --release -p klink-cli --bin klink-bench

echo "==> 2. Building release CLI binary"
cargo build --release -p klink-cli

echo "==> 3. Building Linux Desktop AppImage & .deb via Tauri v2"
cd "$ROOT_DIR/apps/desktop/ui"
npm install
node node_modules/esbuild/install.js || true
cd "$ROOT_DIR/apps/desktop"
./ui/node_modules/.bin/tauri build

mkdir -p "$ROOT_DIR/dist/linux"
cp "$ROOT_DIR/target/release/klink-cli" "$ROOT_DIR/dist/linux/"
find "$ROOT_DIR/apps/desktop/src-tauri/target/release/bundle" -maxdepth 2 -type f \( -name "*.AppImage" -o -name "*.deb" \) -exec cp {} "$ROOT_DIR/dist/linux/" \;
echo "==> Done! Linux release artifacts in dist/linux/"
ls -lh "$ROOT_DIR/dist/linux/"
