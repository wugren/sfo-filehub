#!/usr/bin/env bash
# external-negative 契约检查：旧 `Principal::Token` 形状（缺少 project_scope）
# 必须被编译器拒绝。
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR"; while [ ! -d .git ]; do cd ..; done; pwd)
export CARGO_HOME="$REPO_ROOT/.harness/cargo-home"
export CARGO_TARGET_DIR="$REPO_ROOT/.harness/cargo-target"
mkdir -p "$CARGO_HOME"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/neg/src"

cat > "$TMP/neg/Cargo.toml" <<EOF
[package]
name = "negative-old-token-principal"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
filehub-server = { path = "$REPO_ROOT/server" }
EOF

cat > "$TMP/neg/src/main.rs" <<'EOF'
use filehub_server::model::{Principal, Scope, ScopeSet, TokenId, UserId};
use std::collections::HashSet;

fn main() {
    let mut scopes = HashSet::new();
    scopes.insert(Scope::MetadataRead);
    // 旧形状：Principal::Token 必须携带 project_scope，缺该字段应无法编译。
    let _ = Principal::Token {
        token_id: TokenId(1),
        scopes: ScopeSet(scopes),
        user_id: UserId(1),
    };
}
EOF

cd "$TMP/neg"
if cargo check > check.log 2>&1; then
  echo "negative fixture: expected compile failure for old Principal::Token shape" >&2
  exit 1
fi
if ! rg -q "missing field .project_scope." check.log; then
  cat check.log >&2
  echo "negative fixture: failure did not reference missing field project_scope" >&2
  exit 1
fi
echo "external-negative: passed (old Principal::Token rejected)"
