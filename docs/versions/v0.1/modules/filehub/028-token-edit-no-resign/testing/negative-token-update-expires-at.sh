#!/usr/bin/env bash
# 028 external-negative：旧字段 expires_at 必须编译失败，且错误必须来自该字段。
set -u
ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)" || exit 2
FIXTURE="docs/versions/v0.1/modules/filehub/028-token-edit-no-resign/testing/negative-token-update-expires-at.ts"
TSC="$ROOT/admin-web/node_modules/.bin/tsc"
if [ ! -x "$TSC" ]; then
  echo "admin-web tsc not found; run npm install first" >&2
  exit 2
fi
OUTPUT="$("$TSC" --noEmit --strict "$ROOT/$FIXTURE" 2>&1)"
STATUS=$?
if [ "$STATUS" -eq 0 ]; then
  echo "expected compile failure for removed expires_at field, but tsc passed" >&2
  exit 1
fi
if ! printf '%s' "$OUTPUT" | grep -q "expires_at"; then
  echo "tsc failed for an unexpected reason; expected a diagnostics about expires_at" >&2
  printf '%s\n' "$OUTPUT" >&2
  exit 1
fi
exit 0
