#!/usr/bin/env bash
# external-negative 契约检查：旧 `FilehubClient::publish` 符号必须被编译器拒绝。
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR"; while [ ! -d .git ]; do cd ..; done; pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/neg/src"

cat > "$TMP/neg/Cargo.toml" <<EOF
[package]
name = "negative-old-publish"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
filehub-cli = { path = "$REPO_ROOT/cli" }
EOF

cat > "$TMP/neg/src/main.rs" <<'EOF'
use filehub_cli::apiclient::{Config, FilehubClient};
use std::path::Path;

fn main() {
    let client = FilehubClient::new(Config::default()).expect("client");
    // 旧 v1 契约方法已被 publish_app 取代：该调用必须无法编译。
    let _ = client.publish("B", 1, "1.0.0", Path::new("x.tar.gz"), "sha256");
}
EOF

cd "$TMP/neg"
if cargo check > check.log 2>&1; then
  echo "negative fixture: expected compile failure for removed symbol 'publish'" >&2
  exit 1
fi
if ! rg -q "no (method|function|associated item) named .publish." check.log; then
  cat check.log >&2
  echo "negative fixture: failure did not reference removed symbol 'publish'" >&2
  exit 1
fi
echo "external-negative: passed (removed publish method rejected)"
