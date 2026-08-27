#!/bin/sh
# filehub 容器入口：把环境变量翻译成 YAML 配置并拉起 server + nginx。
# 数据目录固定为容器内 /data（db_path=/data/filehub.db，files=/data/files），
# 外部持久化位置由 Docker -v /--mount 卷挂载决定，不提供数据目录环境变量。
set -eu

DATA_DIR=/data
FILES_DIR="$DATA_DIR/files"
CONFIG_DIR="${FH_CONFIG_DIR:-/etc/filehub}"
CONFIG_PATH="${FH_CONFIG:-$CONFIG_DIR/filehub-server.yaml}"
FH_SERVER_PORT="${FH_SERVER_PORT:-8080}"
FH_ADMIN_USERNAME="${FH_ADMIN_USERNAME-admin}"
FH_ADMIN_PASSWORD="${FH_ADMIN_PASSWORD-change-me}"
FH_MAX_ARCHIVE_BYTES="${FH_MAX_ARCHIVE_BYTES:-104857600}"
FH_LOGIN_RATE_LIMIT_PER_MINUTE="${FH_LOGIN_RATE_LIMIT_PER_MINUTE:-30}"
FH_LOGIN_RATE_LIMIT_WINDOW_SECS="${FH_LOGIN_RATE_LIMIT_WINDOW_SECS:-60}"

fail() {
    echo "filehub-entrypoint: $*" >&2
    exit 1
}

if [ -z "$FH_ADMIN_USERNAME" ]; then
    fail "FH_ADMIN_USERNAME must not be empty"
fi
if [ -z "$FH_ADMIN_PASSWORD" ]; then
    fail "FH_ADMIN_PASSWORD must not be empty"
fi

case "$FH_SERVER_PORT" in
    ''|*[!0-9]*)
        fail "FH_SERVER_PORT must be an integer, got: $FH_SERVER_PORT" ;;
esac
if [ "$FH_SERVER_PORT" -lt 1 ] || [ "$FH_SERVER_PORT" -gt 65535 ]; then
    fail "FH_SERVER_PORT must be between 1 and 65535, got: $FH_SERVER_PORT"
fi
case "$FH_MAX_ARCHIVE_BYTES" in
    ''|*[!0-9]*)
        fail "FH_MAX_ARCHIVE_BYTES must be an integer, got: $FH_MAX_ARCHIVE_BYTES" ;;
esac
if [ "$FH_MAX_ARCHIVE_BYTES" -lt 1 ]; then
    fail "FH_MAX_ARCHIVE_BYTES must be at least 1, got: $FH_MAX_ARCHIVE_BYTES"
fi
case "$FH_LOGIN_RATE_LIMIT_PER_MINUTE" in
    ''|*[!0-9]*)
        fail "FH_LOGIN_RATE_LIMIT_PER_MINUTE must be an integer, got: $FH_LOGIN_RATE_LIMIT_PER_MINUTE" ;;
esac
case "$FH_LOGIN_RATE_LIMIT_WINDOW_SECS" in
    ''|*[!0-9]*)
        fail "FH_LOGIN_RATE_LIMIT_WINDOW_SECS must be an integer, got: $FH_LOGIN_RATE_LIMIT_WINDOW_SECS" ;;
esac
if [ "$FH_LOGIN_RATE_LIMIT_WINDOW_SECS" -lt 1 ]; then
    fail "FH_LOGIN_RATE_LIMIT_WINDOW_SECS must be at least 1, got: $FH_LOGIN_RATE_LIMIT_WINDOW_SECS"
fi

mkdir -p "$FILES_DIR" "$CONFIG_DIR"

# FH_SESSION_PRIVATE_KEY 缺省时生成 Ed25519 PKCS#8 PEM 私钥并持久化，
# 重启后复用同一密钥，既有 session/refresh JWT 仍可验签。
if [ -z "${FH_SESSION_PRIVATE_KEY:-}" ]; then
    SESSION_PRIVATE_KEY_FILE="$DATA_DIR/.session_private_key.pem"
    if [ -s "$SESSION_PRIVATE_KEY_FILE" ]; then
        FH_SESSION_PRIVATE_KEY="$(cat "$SESSION_PRIVATE_KEY_FILE")"
    else
        umask 077
        openssl genpkey -algorithm Ed25519 -out "$SESSION_PRIVATE_KEY_FILE"
        chmod 600 "$SESSION_PRIVATE_KEY_FILE"
        FH_SESSION_PRIVATE_KEY="$(cat "$SESSION_PRIVATE_KEY_FILE")"
        echo "filehub-entrypoint: generated Ed25519 session private key at $SESSION_PRIVATE_KEY_FILE" >&2
    fi
fi

if [ "$FH_ADMIN_PASSWORD" = "change-me" ]; then
    echo "filehub-entrypoint: WARNING FH_ADMIN_PASSWORD is the default 'change-me'; set a strong password for any exposed deployment" >&2
fi

umask 077
jq -n -r \
    --arg server_addr 127.0.0.1 \
    --argjson server_port "$FH_SERVER_PORT" \
    --arg session_private_key "$FH_SESSION_PRIVATE_KEY" \
    --arg admin_username "$FH_ADMIN_USERNAME" \
    --arg admin_password "$FH_ADMIN_PASSWORD" \
    --arg db_path "$DATA_DIR/filehub.db" \
    --arg files_dir "$FILES_DIR" \
    --argjson max_archive_bytes "$FH_MAX_ARCHIVE_BYTES" \
    --argjson login_rate_limit_per_minute "$FH_LOGIN_RATE_LIMIT_PER_MINUTE" \
    --argjson login_rate_limit_window_secs "$FH_LOGIN_RATE_LIMIT_WINDOW_SECS" \
    '"server:\n" +
     "  server_addr: \($server_addr | @json)\n" +
     "  port: \($server_port)\n" +
     "  allow_origins: [\"*\"]\n" +
     "  allow_methods: [\"GET\", \"POST\", \"PUT\", \"DELETE\", \"OPTIONS\"]\n" +
     "  allow_headers: [\"*\"]\n" +
     "  expose_headers: []\n" +
     "  max_age: 3600\n" +
     "  support_credentials: false\n" +
     "  login_rate_limit_per_minute: \($login_rate_limit_per_minute)\n" +
     "  login_rate_limit_window_secs: \($login_rate_limit_window_secs)\n" +
     "users:\n" +
     "  session_private_key: \($session_private_key | @json)\n" +
     "  users:\n" +
     "    - username: \($admin_username | @json)\n" +
     "      password: \($admin_password | @json)\n" +
     "files:\n" +
     "  data_dir: \($files_dir | @json)\n" +
     "  max_archive_bytes: \($max_archive_bytes)\n" +
     "db_path: \($db_path | @json)\n"' > "$CONFIG_PATH"

# 用 FH_SERVER_PORT 替换 nginx 站点模板并做语法校验，避免配置漂移。
sed "s/__SERVER_PORT__/$FH_SERVER_PORT/g" /etc/nginx/filehub.conf.tpl \
    > /etc/nginx/conf.d/filehub.conf
nginx -t

/usr/local/bin/filehub-server "$CONFIG_PATH" &
SERVER_PID=$!

cleanup() {
    trap - INT TERM EXIT
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
}
trap cleanup INT TERM EXIT

nginx -g 'daemon off;'
status=$?
cleanup
exit "$status"
