# filehub Docker 镜像（server + admin-web 一体）

一个镜像内容器同时运行 `filehub-server` 与 nginx（admin-web 静态站点 + API 反代）。
容器启动后直接访问管理页面即可使用；镜像不终结 HTTPS，对外加密由前置反向代理处理。

## 构建（本地编译 + 生成镜像）

镜像内不进行编译；请先在本地执行根目录脚本 `./build-docker.sh`，它会依次：
检查工具链（docker/cargo/npm）→ `cargo build --release -p filehub-server`
→ `VITE_API_BASE_URL=/ npm run build`（`node_modules` 缺失时才先 `npm ci`）
→ 组装最小构建上下文 → `docker build -t filehub:dev`，结束后自动清理临时
上下文。

前置要求：本机装有 Docker、Rust（cargo）与 Node.js/npm。

```bash
./build-docker.sh
```

自定义镜像 tag（默认 `filehub:dev`）：

```bash
IMAGE_TAG=filehub:0.1.0 ./build-docker.sh
```

生成的镜像内包含：

- `filehub-server` release 二进制（容器内监听 `127.0.0.1:${FH_SERVER_PORT}`，默认 8080）；
- admin-web 构建产物（`VITE_API_BASE_URL=/`，与 API 同源）；
- nginx 站点配置：`/account/`、`/api/v1/` 反代到 server，其余路径服务管理页。

## 运行

数据目录在容器内固定为 `/data`（SQLite 为 `/data/filehub.db`，文件归档为
`/data/files`），外部持久化位置请用 `-v` 卷挂载指定，不使用数据目录环境变量：

```bash
mkdir -p ~/filehub-data
docker run -d --name filehub \
  -p 8080:80 \
  -v ~/filehub-data:/data \
  -e FH_SESSION_KEY="$(openssl rand -hex 32)" \
  -e FH_ADMIN_PASSWORD="请换成强密码" \
  filehub:dev
```

浏览器访问 `http://127.0.0.1:8080`，使用 `admin` 与所设密码登录。
重建容器时挂载同一个 `-v` 路径，数据库与文件归档都会保留。

不显式设置 `FH_SESSION_KEY` 时，入口脚本会在 `/data/.session_key` 生成并持久化
随机密钥，重启后仍可续期会话；该文件属于敏感数据，请随数据卷一起备份。

## 环境变量

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `FH_SERVER_PORT` | `8080` | 容器内 server 监听端口，nginx 反代目标 |
| `FH_SESSION_KEY` | 自动生成并持久化到 `/data/.session_key` | 会话签名密钥；生产环境建议显式设置 |
| `FH_ADMIN_USERNAME` | `admin` | 初始 owner 账号 |
| `FH_ADMIN_PASSWORD` | `change-me` | 初始密码；启动时会告警，生产必须显式设置 |
| `FH_MAX_ARCHIVE_BYTES` | `104857600` | 单个归档上传上限（nginx 不设上限，由 server 约束） |

## docker compose 示例

```yaml
services:
  filehub:
    image: filehub:dev
    ports:
      - "8080:80"
    volumes:
      - filehub-data:/data
    environment:
      FH_SESSION_KEY: ${FH_SESSION_KEY}
      FH_ADMIN_PASSWORD: ${FH_ADMIN_PASSWORD}

volumes:
  filehub-data:
```

## 运维提示

- 首次启动前把 `/data` 卷备份视为数据库与归档备份；推荐同时备份
  `/data/.session_key`。
- 容器默认以 root 启动以便读写挂载卷；以 `--user` 运行时，请确保挂载目录对
  指定 UID/GID 可写。
- 健康检查固定访问 `http://127.0.0.1/healthz`。
- 对外 HTTPS、限流与防火墙策略不在镜像内实现，请在反向代理/网关层配置。
