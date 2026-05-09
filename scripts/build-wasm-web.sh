#!/usr/bin/env bash
# Эквивалент `wasm-pack build --target web` для крейта blog-wasm: собирает WASM и генерирует JS в blog-wasm/pkg/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export TMPDIR="${TMPDIR:-$ROOT/tmp}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/tmp/target}"
mkdir -p "$TMPDIR" "$CARGO_TARGET_DIR"

cd "$ROOT"
cargo build -p blog-wasm --target wasm32-unknown-unknown --release

OUT_WASM="$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/blog_wasm.wasm"
rm -rf "$ROOT/blog-wasm/pkg"
mkdir -p "$ROOT/blog-wasm/pkg"

wasm-bindgen "$OUT_WASM" \
  --out-dir "$ROOT/blog-wasm/pkg" \
  --target web \
  --no-typescript

echo "Done: $ROOT/blog-wasm/pkg/ (load index.html from $ROOT)"
