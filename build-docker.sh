#!/bin/sh
# build-docker.sh：本地编译 filehub-server 与 admin-web，再生成 filehub Docker 镜像。
# Docker 镜像内不进行任何编译，只打包本脚本产出的二进制与静态文件。
set -eu

IMAGE_TAG="${IMAGE_TAG:-filehub:dev}"

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT_DIR"

for tool in docker cargo npm; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "build-docker: missing required command: $tool" >&2
        exit 1
    fi
done

echo "==> [1/4] build filehub-server (release)"
cargo build --release -p filehub-server

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
cp target/release/filehub-server "$CONTEXT_DIR/server/filehub-server"
cp -R admin-web/dist/. "$CONTEXT_DIR/web/"

echo "==> [4/4] docker build -t $IMAGE_TAG"
docker build -t "$IMAGE_TAG" "$CONTEXT_DIR"

echo "==> done: $IMAGE_TAG"
