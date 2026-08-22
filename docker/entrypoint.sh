#!/bin/sh
# filehub 容器入口：把环境变量翻译成 filehub-server 配置并拉起 server + nginx。
# 数据目录固定为容器内 /data（db_path=/data/filehub.db，files=/data/files），
# 外部持久化位置由 Docker -v /--mount 卷挂载决定，不提供数据目录环境变量。
set -eu

DATA_DIR=/data
FILES_DIR="$DATA_DIR/files"
CONFIG_DIR="${FH_CONFIG_DIR:-/etc/filehub}"
CONFIG_PATH="${FH_CONFIG:-$CONFIG_DIR/filehub-server.json}"
FH_SERVER_PORT="${FH_SERVER_PORT:-8080}"
FH_ADMIN_USERNAME="${FH_ADMIN_USERNAME-admin}"
FH_ADMIN_PASSWORD="${FH_ADMIN_PASSWORD-change-me}"
FH_MAX_ARCHIVE_BYTES="${FH_MAX_ARCHIVE_BYTES:-104857600}"

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

mkdir -p "$FILES_DIR" "$CONFIG_DIR"

# FH_SESSION_KEY 缺省时生成随机密钥并持久化到数据目录，重启后会话仍可续期。
if [ -z "${FH_SESSION_KEY:-}" ]; then
    SESSION_KEY_FILE="$DATA_DIR/.session_key"
    if [ -s "$SESSION_KEY_FILE" ]; then
        FH_SESSION_KEY="$(cat "$SESSION_KEY_FILE")"
    else
        umask 077
        FH_SESSION_KEY="$(head -c 32 /dev/urandom | od -An -v -tx1 | tr -d ' \n')"
        printf '%s\n' "$FH_SESSION_KEY" > "$SESSION_KEY_FILE"
        chmod 600 "$SESSION_KEY_FILE"
        echo "filehub-entrypoint: generated session key at $SESSION_KEY_FILE" >&2
    fi
fi

if [ "$FH_ADMIN_PASSWORD" = "change-me" ]; then
    echo "filehub-entrypoint: WARNING FH_ADMIN_PASSWORD is the default 'change-me'; set a strong password for any exposed deployment" >&2
fi

umask 077
jq -n \
    --arg server_addr 127.0.0.1 \
    --argjson server_port "$FH_SERVER_PORT" \
    --arg session_key "$FH_SESSION_KEY" \
    --arg admin_username "$FH_ADMIN_USERNAME" \
    --arg admin_password "$FH_ADMIN_PASSWORD" \
    --arg db_path "$DATA_DIR/filehub.db" \
    --arg files_dir "$FILES_DIR" \
    --argjson max_archive_bytes "$FH_MAX_ARCHIVE_BYTES" \
    '{
        server: {
            server_addr: $server_addr,
            port: $server_port,
            allow_origins: ["*"],
            allow_methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            allow_headers: ["*"],
            expose_headers: [],
            max_age: 3600,
            support_credentials: false
        },
        users: {
            session_key: $session_key,
            users: [{ username: $admin_username, password: $admin_password, role: "owner" }]
        },
        files: {
            data_dir: $files_dir,
            max_archive_bytes: $max_archive_bytes
        },
        db_path: $db_path
    }' > "$CONFIG_PATH"

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
