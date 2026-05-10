#!/usr/bin/env bash
# blog-server подхватывает .env из каталога workspace (см. dotenvy в blog-server).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
exec cargo run -q -p blog-server
