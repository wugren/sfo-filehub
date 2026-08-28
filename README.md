# filehub（文件集散）

filehub 用于把构建产物以 `.tar.gz` 归档发布到项目版本中：Web 管理后台负责
项目/版本/文件的管理与下载，命令行客户端 `filehub` 负责发布与拉取。三个交付面
共用一份 v1 API 契约（`docs/api/v1-contract.md`），认证统一走
`Authorization` 请求头。

正式版本由 GitHub Actions 构建并发布：推 `v*` 标签（如 `v0.1.0`）后，CI 自动
执行构建与测试，并把**发布产物**（GitHub Release 归档 + GHCR 镜像）交付出来。
下面的快速开始都基于这些发布产物，源码构建仅供开发者/贡献者使用。

## 快速开始（基于 GitHub Actions 发布产物）

### 方式一：Docker 镜像（推荐，含服务端 + 管理后台）

镜像由 CI 发布到 GHCR：`ghcr.io/wugren/filehub:v<版本>`
（当前 workspace 版本为 `0.1.0`，GitHub 仓库为
[wugren/sfo-filehub](https://github.com/wugren/sfo-filehub)）。拉取并运行：

```bash
docker pull ghcr.io/wugren/filehub:v0.1.0
curl -fsSLo filehub-server.yaml \
  https://raw.githubusercontent.com/wugren/sfo-filehub/main/docker/filehub-server.example.yaml
# 替换 YAML 中的示例 Ed25519 私钥与管理员密码后：
chmod 600 ./filehub-server.yaml
mkdir -p ./filehub-data
docker run -d --name filehub \
  -p 8080:80 \
  --mount type=bind,src="$(pwd)/filehub-server.yaml",dst=/etc/filehub/filehub-server.yaml,readonly \
  --mount type=bind,src="$(pwd)/filehub-data",dst=/data \
  ghcr.io/wugren/filehub:v0.1.0
```

容器内同时运行服务端与 nginx（管理页面 + `/account/`、`/api/v1/` 反代），
浏览器打开 `http://127.0.0.1:8080`，用 `admin` 登录。数据目录固定在容器内
`/data`（数据库与文件归档由卷挂载持久化）。镜像只读取挂载到
`/etc/filehub/filehub-server.yaml` 的完整 YAML，不通过环境变量生成或覆盖配置；
容器内 server 地址/端口固定要求为 `127.0.0.1:8080`。配置与升级细节见
[docker/README.md](docker/README.md)。

### 方式二：GitHub Release 归档

打开 [wugren/sfo-filehub 的 Releases 页面](https://github.com/wugren/sfo-filehub/releases)，
选择对应版本的发布，下载适合你的平台 / 场景的归档：

| 归档 | 内容 |
|------|------|
| `filehub-server_<版本>_linux_x86_64.tar.gz` | Linux 服务端二进制 + admin-web 静态产物（手动部署） |
| `filehub-cli_<版本>_linux-x86_64.tar.gz` | Linux CLI 二进制 `filehub` |
| `filehub-cli_<版本>_macos-aarch64.tar.gz` | macOS（Apple Silicon）CLI 二进制 `filehub` |
| `filehub-cli_<版本>_windows-x86_64.tar.gz` | Windows CLI 二进制 `filehub.exe` |

CLI 解压即用：

```bash
tar -xzf filehub-cli_0.1.0_linux-x86_64.tar.gz
./filehub --help
```

服务端 + 管理后台归档的目录结构为 `server/filehub-server` 与 `web/`（admin-web
静态产物）。运行服务端前需按 `server/config.example.yaml` 创建配置（至少替换
`users.session_private_key` 示例私钥与账号密码），再用 nginx/静态服务器托管 `web/` 并将
`/account/`、`/api/v1/` 反代到服务端；nginx 参考配置见
[docker/nginx.conf](docker/nginx.conf)。如果不想自己组装 nginx，直接用方式一的
镜像即可。

## 使用操作

### Web 管理后台

通过方式一（镜像）或本地开发服务（见文末「开发者/本地构建」）打开管理页面，
登录后按以下路径操作：

1. **项目**：新建项目并选择可见性（表单默认 public，可选 private；详情页可随时
   切换）；
2. **版本**：在项目详情页「创建版本」，例如 `1.0.0`；
3. **发布 app**：填写 app 名并选择要发布的归档文件上传（SHA-256 由页面自动计算，
   重复上传同名 app 即为更新，版本锁定后不可再改）；
4. **下载/锁定**：对 app 下载归档；确认版本稳定后「锁定版本」（不可逆）；
5. **访问管理**：成员页按用户编号添加协作者（read/write/admin），Token 页可为
   CLI 创建项目级 token。

### CLI 发布与下载

先登录（密码交互输入，不会出现在命令行参数里；也可用 `--token-stdin` 登录）：

```bash
./filehub login 127.0.0.1:8080 -u admin
```

典型发布/下载工作流：

```bash
# 创建版本（已存在会报 409）
./filehub new-version 127.0.0.1:8080/demo/1.0.0

# 发布：目录或文件会被安全打包为 .tar.gz，客户端自动计算并校验 SHA-256
./filehub push 127.0.0.1:8080/demo/1.0.0/web-app ./release-output

# 查看项目版本
./filehub versions 127.0.0.1:8080/demo

# 下载：校验 SHA-256 后原子落盘
./filehub pull 127.0.0.1:8080/demo/1.0.0/web-app ./web-app.tar.gz

# 版本验证无误后锁定（不可逆；锁定后再 push 会返回 409）
./filehub lock-version 127.0.0.1:8080/demo/1.0.0
```

删除 app 用 `delete-app <server/project/version/name>`。目标串统一为
`host[:port]/project/version/name`；完整命令面、凭据输入方式与稳定退出码见
[cli/README.md](cli/README.md)。

## GitHub Actions 发布说明

发布工作流见 [.github/workflows/build.yml](.github/workflows/build.yml)：

- **推 `v*` 标签**：编译 + 全量测试 + 生成上面 4 个 Release 归档 +
  推送 GHCR 镜像 `ghcr.io/wugren/filehub:v<版本>`；标签必须与 workspace
  版本一致（如 `v0.1.0`），且只在规范仓库 `wugren/sfo-filehub` 执行发布；
- **手动触发（workflow_dispatch）**：仅构建并上传临时 workflow artifact
  （保留 14 天），不创建 Release、不推送镜像；
- CI 只在 Linux 上对 Rust workspace 运行全量测试，三平台都会构建 CLI。

## 测试

```bash
# 统一测试入口（Linux/macOS，需 uv；Windows 用 test-run.bat）
./test-run.sh all all
# 或按交付面
cargo test -p filehub-server
cargo test -p filehub-cli
cd admin-web && npm run test:unit && npm run test:integration
```

## 配置与安全提醒

- **会话签名私钥**：账号 session/refresh JWT 使用 EdDSA（Ed25519）签名；
  `users.session_private_key` 必须是 Ed25519 PKCS#8 PEM，公钥由服务端自动派生。
  示例私钥只能用于本地演示，生产必须用
  `openssl genpkey -algorithm Ed25519` 重新生成并安全写入挂载的 YAML；镜像不会
  自动生成私钥，运维细节见 [docker/README.md](docker/README.md)；
- **升级影响**：从旧 HMAC `users.session_key` 切换后，已签发的 session/refresh
  JWT 无法继续验签，用户需要重新登录；不提供双算法兼容窗口；
- **初始账号**：Docker 示例中的管理员密码只是占位，首次部署必须改密码或使用
  `password_hash`；
- **上传校验**：上传先鉴权后收流，`sha256` 为必填字段，服务端流式复算校验并
  实时执行 `max_archive_bytes` 限长；服务端不解压/不校验归档内容，归档按
  `.tar.gz` 语义存储（CLI 打包/拉取时会校验 gzip 魔数与 SHA-256）；Docker 部署
  时 nginx 对上传/下载请求体不设上限；
- **凭据传输**：所有接口使用 `Authorization: Bearer <session|refresh|token-jwt>`，
  不使用 cookie；密码/token/session 不应出现在 CLI 命令行参数中；
- **登录限流**：应用层默认 30 次/分钟/IP，Docker 内 nginx 对登录路由另有
  5r/s（突发 20）限流；
- **TLS**：server 与镜像都不终结 HTTPS，对外开放时由前置反向代理负责。

## 开发者/本地构建

开发者从源码运行或贡献时，可跳过发布产物自行构建：

```bash
# 服务端
cargo build --release -p filehub-server
cp server/config.example.yaml filehub-server.yaml
# 编辑配置后
./target/release/filehub-server filehub-server.yaml

# 管理后台（开发模式）
cd admin-web && npm ci && npm run dev   # 打开 http://localhost:5173

# CLI
cargo build --release --manifest-path cli/Cargo.toml
./target/release/filehub --help
```

## 相关文档

- 模块边界与交付面职责：[docs/modules/filehub.md](docs/modules/filehub.md)
- v1 API 契约：[docs/api/v1-contract.md](docs/api/v1-contract.md)
- CLI 完整命令面与退出码：[cli/README.md](cli/README.md)
- Docker 镜像说明：[docker/README.md](docker/README.md)
- 发布工作流：[.github/workflows/build.yml](.github/workflows/build.yml)
- 变更记录：[docs/changes/](docs/changes/)
