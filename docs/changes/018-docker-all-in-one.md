# filehub 统一 Docker 镜像（server + admin-web 一体运行）

- Status: complete
- Owner module: filehub（文件集散 v0.1）
- Task manifest: `docs/versions/v0.1/modules/filehub/018-docker-all-in-one/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/018-docker-all-in-one/proposal.md`
- Affected paths: `Dockerfile`、`.dockerignore`、`docker/nginx.conf`、`docker/entrypoint.sh`、`docker/README.md`、`docs/changes/018-docker-all-in-one.md`
- Explicit tier override: none（用户确认 standard）
- Expanded high-risk packet: none

## Approach

1. 根目录多阶段 `Dockerfile`：node:22-alpine 阶段用 `npm ci` 构建 admin-web
   （构建参数 `VITE_API_BASE_URL=/`，页面与 API 同源，不修改源码默认值）；
   rust:1.96-bookworm 阶段构建 `filehub-server` release 二进制；最终
   nginx:1.27-alpine 镜像放入静态产物、二进制、nginx 站点模板与入口脚本，
   镜像内固定数据目录 `/data`。
2. `docker/nginx.conf` 是带 `__SERVER_PORT__` 占位符的站点模板：80 端口提供
   admin-web 静态页面与 SPA 回退，`/account/`、`/api/v1/` 原样反代到容器内
   server；`client_max_body_size 0`，上传体上限由
   `FH_MAX_ARCHIVE_BYTES` 决定。
3. `docker/entrypoint.sh` 用 `jq` 把环境变量安全地生成 server 配置：
   `db_path=/data/filehub.db`、`files.data_dir=/data/files` 固定写死；
   `FH_SESSION_KEY` 缺省时生成 64 位 hex 并持久化到 `/data/.session_key`
   （0600，重启复用）；管理员账号密码、内部端口与归档上限可配；替换 nginx
   模板后先 `nginx -t` 再启动 nginx + server，任一进程退出时清理另一个。
4. `.dockerignore` 排除 target/node_modules/dist/本地 `.env*`/harness 等，
   保证镜像上下文干净且不携带本地密钥。
5. `docker/README.md` 提供构建、`-v` 卷挂载运行、docker compose、环境变量表
   与运维提示（默认密码告警、无 TLS、`--user` 运行要求）。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no（不改 server schema；SQLite
  与文件归档布局仅在容器内数据卷中变化）
- Security, privacy, or trust-boundary change: **yes**——容器默认 root 运行、
  初始密码 `change-me` 与自动生成的会话密钥属于新引入的凭据面；启动日志与
  README 均告警，会话密钥文件以 0600 落在数据卷。
- Concurrency, lifecycle, or runtime integration change: **yes**——nginx 与
  filehub-server 双进程在入口脚本内拉起并清理，健康检查覆盖页面，容器退出
  策略由 Docker 重启策略接管。
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: **yes**——新增镜像构建链与容器交付面；该面此前不存在，属
  加法式交付，不改变现有本地构建/测试链，也没有既有部署需要兼容或回滚。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

以上 yes 项均处于用户确认的提案范围内，未改变需求/范围/验收边界，按已确认
standard 层级执行并记录残余风险。

## Verification

- Targeted check:
  - `cargo build --release -p filehub-server`（通过，释放产物成功生成；仅有
    存量 server 未使用 import/死代码警告，非本次改动引入）；
  - `cd admin-web && VITE_API_BASE_URL=/ npm run build`（通过，dist 正常产出）；
  - `sh -n docker/entrypoint.sh`（通过）；
  - 入口脚本本地仿真（/tmp 内真实 jq + nginx/filehub-server shim）：生成配置
    的 `db_path`/`files.data_dir` 固定指向数据目录、带引号密码 JSON 转义正确、
    nginx 模板按 `FH_SERVER_PORT=9090` 替换、会话密钥 64 hex 生成并跨两次启动
    复用、非法端口/空管理员用户名被拒绝、进程退出后无残留；
  - 真实链路：用入口生成的配置启动 release server，`POST /account/login` 返回
    HTTP 200 且 `err=0`、session 非空；
  - Dockerfile/.dockerignore 静态核对：COPY 目标文件均存在，构建上下文排除项
    与镜像需要一致。
- Result: passed
- Residual risk or follow-up: 本环境没有 Docker 守护进程，`docker build` 与
  `docker run` 冒烟未在本机执行；镜像内入口每次启动都会 `nginx -t` 兜底站点
  语法，建议在 CI 或有 Docker 的机器补一次镜像构建 + 启动 + `-v` 数据持久化
  冒烟。
