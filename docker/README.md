# filehub Docker 镜像（server + admin-web 一体）

一个镜像内容器同时运行 `filehub-server` 与 nginx（admin-web 静态站点 + API 反代）。
容器启动后直接访问管理页面即可使用；镜像不终结 HTTPS，对外加密由前置反向代理处理。

## 构建（本地编译 + 生成镜像）

镜像内不进行编译；请先在本地执行根目录脚本 `./build-docker.sh`，它会依次：
检查工具链（docker/cargo/rustup/npm，musl 目标缺失时自动 `rustup target
add x86_64-unknown-linux-musl`）→
`cargo build --release -p filehub-server --target x86_64-unknown-linux-musl`
→ `VITE_API_BASE_URL=/ npm run build`（`node_modules` 缺失时才先 `npm ci`）
→ 组装最小构建上下文 → `docker build -t filehub:dev`，结束后自动清理临时
上下文。

前置要求：本机装有 Docker、Rust（rustup 管理的 cargo）与 Node.js/npm。
Linux 服务端固定为 musl 静态链接：脚本会自动添加 rustup musl 目标；若 C
依赖的编译/链接环节报缺少 `musl-gcc`（Debian/Ubuntu 安装 `musl-tools`），
按提示安装后重试。musl 静态产物可直接运行在 `nginx:alpine`（musl libc）
基础镜像上，镜像内无需 gcompat 或 glibc loader。

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

入口脚本在 `/etc/filehub/filehub-server.yaml` 生成服务端 YAML 配置（可通过既有
`FH_CONFIG` 覆盖路径），再用该文件启动 `filehub-server`。

## 运行

数据目录在容器内固定为 `/data`（SQLite 为 `/data/filehub.db`，文件归档为
`/data/files`），外部持久化位置请用 `-v` 卷挂载指定，不使用数据目录环境变量：

```bash
mkdir -p ~/filehub-data
docker run -d --name filehub \
  -p 8080:80 \
  -v ~/filehub-data:/data \
  -e FH_ADMIN_PASSWORD="请换成强密码" \
  filehub:dev
```

浏览器访问 `http://127.0.0.1:8080`，使用 `admin` 与所设密码登录。
重建容器时挂载同一个 `-v` 路径，数据库与文件归档都会保留。

不显式设置 `FH_SESSION_PRIVATE_KEY` 时，入口脚本会在
`/data/.session_private_key.pem` 生成并以 `0600` 持久化 Ed25519 PKCS#8 PEM
私钥，重启后仍可验证已有 session/refresh JWT。也可由 secret manager 通过
`FH_SESSION_PRIVATE_KEY` 注入完整 PEM。私钥属于敏感数据，请随数据卷安全备份，
不得写入日志或镜像。

## 环境变量

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `FH_SERVER_PORT` | `8080` | 容器内 server 监听端口，nginx 反代目标 |
| `FH_SESSION_PRIVATE_KEY` | 自动生成并持久化到 `/data/.session_private_key.pem` | Ed25519 PKCS#8 PEM 会话签名私钥；公钥自动派生 |
| `FH_ADMIN_USERNAME` | `admin` | 初始 owner 账号 |
| `FH_ADMIN_PASSWORD` | `change-me` | 初始密码；启动时会告警，生产必须显式设置 |
| `FH_MAX_ARCHIVE_BYTES` | `104857600` | 单个归档上传上限（nginx 不设上限，由 server 约束） |
| `FH_LOGIN_RATE_LIMIT_PER_MINUTE` | `30` | 每个来源 IP 每分钟最多登录尝试（0 关闭应用层限流） |
| `FH_LOGIN_RATE_LIMIT_WINDOW_SECS` | `60` | 应用层登录限流统计窗口（秒） |

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
      # 可选：由 secret manager 注入完整 Ed25519 PKCS#8 PEM；省略则在 /data 自动生成。
      FH_SESSION_PRIVATE_KEY: ${FH_SESSION_PRIVATE_KEY}
      FH_ADMIN_PASSWORD: ${FH_ADMIN_PASSWORD}

volumes:
  filehub-data:
```

## 运维提示

- 首次启动前把 `/data` 卷备份视为数据库与归档备份；推荐同时备份
  `/data/.session_private_key.pem`。丢失或替换私钥会使已有 session/refresh JWT
  全部失效，用户需要重新登录。
- 容器默认以 root 启动以便读写挂载卷；以 `--user` 运行时，请确保挂载目录对
  指定 UID/GID 可写。
- 健康检查固定访问 `http://127.0.0.1/healthz`。
- 登录限流已在镜像内实现：nginx `limit_req`（5r/s、burst 20、超限 429）对
  `location = /account/login` 生效，filehub-server 应用层再按来源 IP 固定窗口
  限流（默认 30 次/分钟，可通过 `FH_LOGIN_RATE_LIMIT_*` 调整）。对外 HTTPS、
  防火墙与其它边缘限流策略仍建议在前置反向代理/网关上配置。
