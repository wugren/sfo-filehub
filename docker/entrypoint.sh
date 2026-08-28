#!/bin/sh
# filehub 容器入口：读取固定位置的 YAML 配置并拉起 server + nginx。
# 配置必须只读挂载到 /etc/filehub/filehub-server.yaml；镜像不生成或改写配置。
set -eu

DATA_DIR=/data
FILES_DIR="$DATA_DIR/files"
CONFIG_PATH=/etc/filehub/filehub-server.yaml

fail() {
    echo "filehub-entrypoint: $*" >&2
    exit 1
}

if [ ! -f "$CONFIG_PATH" ]; then
    fail "required config file not found: $CONFIG_PATH; mount a YAML config at this path"
fi
if [ ! -r "$CONFIG_PATH" ]; then
    fail "config file is not readable: $CONFIG_PATH"
fi

mkdir -p "$FILES_DIR"
nginx -t

/usr/local/bin/filehub-server "$CONFIG_PATH" &
SERVER_PID=$!

cleanup() {
    trap - INT TERM EXIT
    for pid in "${NGINX_PID:-}" "${SERVER_PID:-}"; do
        if [ -n "$pid" ]; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    for pid in "${NGINX_PID:-}" "${SERVER_PID:-}"; do
        if [ -n "$pid" ]; then
            wait "$pid" 2>/dev/null || true
        fi
    done
}
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap cleanup EXIT

nginx -g 'daemon off;' &
NGINX_PID=$!

# 持续监督两个进程，避免 server 配置/端口错误后只留下 nginx 假存活。
while kill -0 "$SERVER_PID" 2>/dev/null && kill -0 "$NGINX_PID" 2>/dev/null; do
    sleep 1
done

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    if wait "$SERVER_PID"; then
        status=0
    else
        status=$?
    fi
    echo "filehub-entrypoint: filehub-server exited with status $status" >&2
else
    if wait "$NGINX_PID"; then
        status=0
    else
        status=$?
    fi
    echo "filehub-entrypoint: nginx exited with status $status" >&2
fi

exit "$status"
