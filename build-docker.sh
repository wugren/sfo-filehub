#!/bin/sh
# build-docker.sh：本地编译 filehub-server 与 admin-web，再生成 filehub Docker 镜像。
# Docker 镜像内不进行任何编译，只打包本脚本产出的二进制与静态文件。
# Linux 服务端固定用 musl 静态链接，避免 Alpine 基础镜像缺少 glibc loader
# 时容器内 filehub-server 启动报 not found。
set -eu

IMAGE_TAG="${IMAGE_TAG:-filehub:dev}"
MUSL_TARGET=x86_64-unknown-linux-musl

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT_DIR"

for tool in docker cargo npm; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "build-docker: missing required command: $tool" >&2
        exit 1
    fi
done

if ! command -v rustup >/dev/null 2>&1; then
    echo "build-docker: missing required command: rustup" >&2
    echo "  install rustup first (https://rustup.rs), then re-run this script" >&2
    exit 1
fi

if ! rustup target list --installed | grep -qx "$MUSL_TARGET"; then
    echo "==> install rustup target $MUSL_TARGET"
    rustup target add "$MUSL_TARGET"
fi

if ! command -v musl-gcc >/dev/null 2>&1; then
    echo "build-docker: warning: musl-gcc (musl-tools) not found" >&2
    echo "  build-docker: 当前依赖已验证无需 musl-gcc 也能链接；若 C 依赖编译或链接报错，" >&2
    echo "  build-docker: Debian/Ubuntu 请先安装 musl-tools 后重试" >&2
fi

echo "==> [1/4] build filehub-server (musl static release)"
cargo build --release -p filehub-server --target "$MUSL_TARGET"

echo "==> [2/4] build admin-web (VITE_API_BASE_URL=/，页面与 API 同源)"
(
    cd admin-web
    if [ -d node_modules ] && [ -x node_modules/.bin/vite ]; then
        echo "     (reuse existing node_modules; run 'npm ci' manually to refresh)"
    else
        npm ci
    fi
    VITE_API_BASE_URL=/ npm run build
)

echo "==> [3/4] assemble minimal docker build context"
CONTEXT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/filehub-image.XXXXXX")"
trap 'rm -rf -- "$CONTEXT_DIR"' EXIT INT TERM

mkdir -p "$CONTEXT_DIR/server" "$CONTEXT_DIR/web"
cp docker/Dockerfile "$CONTEXT_DIR/Dockerfile"
cp docker/nginx.conf "$CONTEXT_DIR/nginx.conf"
cp docker/entrypoint.sh "$CONTEXT_DIR/entrypoint.sh"
cp "target/$MUSL_TARGET/release/filehub-server" "$CONTEXT_DIR/server/filehub-server"
cp -R admin-web/dist/. "$CONTEXT_DIR/web/"

echo "==> [4/4] docker build -t $IMAGE_TAG"
docker build -t "$IMAGE_TAG" "$CONTEXT_DIR"

echo "==> done: $IMAGE_TAG"
