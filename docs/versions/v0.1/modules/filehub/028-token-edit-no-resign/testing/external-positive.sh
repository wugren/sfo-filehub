#!/usr/bin/env bash
# 028 external-positive：新路径编译闭包（服务端 lib/bins + 管理端 tsc/vite build）。
set -e
ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)" || exit 2
cd "$ROOT"
cargo check --manifest-path server/Cargo.toml --lib --bins
cd "$ROOT/admin-web"
npm run build
