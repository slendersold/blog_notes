#!/usr/bin/env bash
# Статическая отдача index.html и blog-wasm/pkg/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${1:-8765}"
cd "$ROOT"
echo "http://127.0.0.1:${PORT}/index.html"
exec python3 -m http.server "$PORT"
