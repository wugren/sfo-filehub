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

- `filehub-server` release 二进制（容器内固定监听 `127.0.0.1:8080`）；
- admin-web 构建产物（`VITE_API_BASE_URL=/`，与 API 同源）；
- nginx 站点配置：`/account/`、`/api/v1/` 固定反代到 `127.0.0.1:8080`，其余路径服务管理页。

镜像不通过环境变量生成配置。启动前必须把完整 YAML 以只读方式挂载到固定位置
`/etc/filehub/filehub-server.yaml`；配置缺失、不可读或导致 server 启动失败时，
容器会失败退出。

## 运行

先复制 Docker 专用示例，替换管理员密码和示例 Ed25519 私钥，并限制文件权限：

```bash
cp docker/filehub-server.example.yaml ./filehub-server.yaml
openssl genpkey -algorithm Ed25519
# 把命令输出的完整 PEM 安全写入 users.session_private_key，再修改账号密码。
chmod 600 ./filehub-server.yaml
mkdir -p ./filehub-data

docker run -d --name filehub \
  --restart unless-stopped \
  -p 8080:80 \
  --mount type=bind,src="$(pwd)/filehub-server.yaml",dst=/etc/filehub/filehub-server.yaml,readonly \
  --mount type=bind,src="$(pwd)/filehub-data",dst=/data \
  filehub:dev
```

浏览器访问 `http://127.0.0.1:8080`，使用 YAML 中配置的账号登录。数据目录在
容器内固定为 `/data`：SQLite 为 `/data/filehub.db`，文件归档为 `/data/files`。
重建容器时继续挂载同一个数据目录，数据库与归档会保留。

Docker 镜像不支持环境变量覆盖 YAML 字段。完整配置项以
[`filehub-server.example.yaml`](filehub-server.example.yaml) 和
[`server/config.example.yaml`](../server/config.example.yaml) 为准。容器内部
`server.server_addr` 必须是 `127.0.0.1`、`server.port` 必须是 `8080`；需要改变
外部端口时修改 `-p` 左侧，例如 `-p 9000:80`。

## docker compose 示例

```yaml
services:
  filehub:
    image: filehub:dev
    ports:
      - "8080:80"
    volumes:
      - filehub-data:/data
      - type: bind
        source: ./filehub-server.yaml
        target: /etc/filehub/filehub-server.yaml
        read_only: true

volumes:
  filehub-data:
```

## 运维提示

- YAML 含管理员密码或密码哈希及会话签名私钥，应使用 secret 管理流程、宿主机
  `0600` 权限和只读挂载；不要提交到源码仓库或输出到日志。
- 从旧环境变量版镜像升级时，若要保留已有 session/refresh JWT，先从旧数据卷读取
  `/data/.session_private_key.pem`，把同一 PEM 安全写入新 YAML；换新或丢失私钥会
  要求所有用户重新登录。回滚时可恢复旧镜像及其原环境变量启动参数，`/data`
  布局不变。
- 备份 `/data` 数据卷以及独立保存的 YAML/secret；两者缺一都不是完整恢复材料。
- 容器默认以 root 启动以便读写挂载卷；以 `--user` 运行时，请确保挂载目录对
  指定 UID/GID 可写。
- 健康检查固定访问 `http://127.0.0.1/healthz`。
- 登录限流已在镜像内实现：nginx `limit_req`（5r/s、burst 20、超限 429）对
  `location = /account/login` 生效，filehub-server 应用层再按来源 IP 固定窗口
  限流（示例为 30 次/分钟，可在 YAML 中调整）。对外 HTTPS、
  防火墙与其它边缘限流策略仍建议在前置反向代理/网关上配置。
