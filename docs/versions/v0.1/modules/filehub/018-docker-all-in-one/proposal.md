---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认；且要求数据目录不使用环境变量，改为固定容器内 `/data`，
  由 Docker 卷挂载（`-v`）指定外部存储位置。

# filehub 统一 Docker 镜像（server + admin-web 一体运行）

Risk profile: not-created（standard 层级不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-22 当前用户回复「确认」，并按数据目录修订意见执行：
    不提供 `FH_DATA_DIR`，容器内数据目录固定 `/data`，外部路径由 `-v` 挂载决定。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：本次新增 Docker 构建与容器运行交付面，产生镜像产物并建立
    容器化运行入口，不是单点修整；
  - 未升级 high-risk：当前仓库没有任何在管容器化部署/发布面，本次是**加法式**
    增加 Dockerfile、nginx 反向代理与入口脚本；不修改 server/admin-web 源码默认
    行为、不修改数据库 schema、不修改 v1 API 契约、不动现有本地构建/测试链；
  - 本次新引入的运行集成（nginx + filehub-server 双进程）、凭据环境变量与固定数据
    目录（`-v` 卷挂载）配置属于可验证且有界的单项目部署能力，按仓库默认以 standard
    覆盖：变更记录 + 定向验证 + 独立缺陷发现后完成报告。
- Proposal and tier confirmation: 2026-08-22 用户「确认」；确认版本即修订后的
  数据目录固定 `/data` + `-v` 挂载版本，层级 standard。

## Background and Goal

用户需要一个把 `filehub-server`（Rust 服务后台）与 `admin-web`（Vite 静态管理
页面）打进同一个 Docker 镜像的交付物：容器启动后直接通过网页访问服务，管理页面与
API 同源；数据（SQLite + 文件归档）统一存放在容器内固定目录 `/data`，外部存放
位置完全通过 Docker 卷挂载（`-v`）指定，不使用数据目录环境变量。

现状：仓库没有任何 Dockerfile/容器运行文件；server 通过 `filehub-server.json` 配置
`files.data_dir`、`db_path` 等路径，admin-web 构建时通过 `VITE_API_BASE_URL` 决定
API 地址。本任务在不动这两处默认行为的前提下，用容器入口脚本把环境变量翻译成
server 配置，并用 nginx 把 `/account/` 与 `/api/v1/` 反代到内部 server。

## Scope

### In scope

1. 根目录新增 `Dockerfile`（多阶段构建）：
   - Node 阶段：`npm ci` + `npm run build`，构建 admin-web 静态产物；
   - Rust 阶段：`cargo build --release -p filehub-server`，构建服务端二进制；
   - 最终阶段：nginx alpine 镜像，放入 admin-web `dist`、server 二进制、
     nginx 站点配置与入口脚本；
2. 新增 `.dockerignore`（排除 target、node_modules、admin-web/dist、data、
   .harness、.env.local 等，保证镜像上下文干净且不携带本地密钥）；
3. 新增 `docker/nginx.conf`：80 端口提供 admin-web 静态页面；`/account/`、
   `/api/v1/` 反代到容器内 `127.0.0.1:<FH_SERVER_PORT>`；SPA 路由回退到
   `index.html`；上传体大小由 server 的 `max_archive_bytes` 约束；
4. 新增 `docker/entrypoint.sh`：用环境变量生成 `filehub-server.json` 并启动
   server + nginx，支持：
   - 数据目录固定为容器内 `/data`（不开放环境变量）：`db_path=/data/filehub.db`、
     `files.data_dir=/data/files`。外部持久化位置由 `-v <host-path>:/data` 指定；
   - `FH_SERVER_PORT`（默认 8080，容器内反代端口）；
   - `FH_SESSION_KEY`（可选：不设则自动生成并持久化到数据目录，
     重启后会话仍可续期）；
   - `FH_ADMIN_USERNAME` / `FH_ADMIN_PASSWORD`（默认 admin / change-me，
     未显式设置时日志告警）；
   - `FH_MAX_ARCHIVE_BYTES`（默认 104857600）；
5. 新增 `docker/README.md`：`docker build`、`docker run -v`、可选
   `docker compose` 示例与配置说明；
6. 按 standard 流程产出 `docs/changes/018-docker-all-in-one.md` 与任务包内
   `completion-report.md`。

### Out of scope

- 不修改 server 源码、`server/Cargo.toml` 依赖、数据库 schema、登录/token 逻辑与
  `docs/api/v1-contract.md`；
- 不修改 admin-web 源码默认 API 行为：镜像构建时仅通过
  `VITE_API_BASE_URL=/` 让页面与 API 同源，不落库改默认值；
- 不在镜像内终结 TLS/HTTPS：对外 HTTPS 由前置反向代理/网关处理；
- 不做多架构镜像、不做镜像仓库推送、不新增 CI/registry 工作流；
- 不开放容器内 server 端口的直接宿主机映射（入口固定为 nginx 80 端口）；
- 不支持多管理员用户的环境变量配置（沿用 server 配置文件能力，用户可挂载完整
  自定义配置文件覆盖入口生成的配置）。

### Boundary with neighboring modules

本任务只增加构建/部署包装层，三个交付面（server/web/cli）的实现模块均不改动；
admin-web 构建参数与 nginx 反代只发生在镜像构建与容器运行边界内。

## Requirement Review

需求合理：单容器交付能显著降低自托管部署成本；「启动后直接页面访问」要求页面与
API 同源，nginx 静态服务 + 反代是文件集散场景的标准形态；固定容器内 `/data` 目录、
由 `-v` 挂载外部持久化位置，既满足用户对「用 `-v` 配置数据目录」的要求，也让
server 配置只依赖一份不会歧义的真实路径。

方案对比与选择：
- **nginx 单容器反代（选定）**：admin-web 是构建期静态站点，server 现有
  `sfo-http` 抽象没有静态资源服务能力；在镜像内加 nginx 不动业务代码，代价是
  多一个进程与 4MB 级镜像体积；
- **Rust 内嵌静态资源**：需要给 server 引入静态文件/embed 能力并扩展
  `sfo-http` 装配，改动业务 crate、依赖与测试面，超出「生成镜像」的诉求；
- **配置注入方式**：入口脚本用 `jq` 生成 JSON 配置文件，避免正则替换的转义
  陷阱；固定 `/data` 同时落到 `db_path` 与 `files.data_dir`，保证数据单目录，外部
  位置只经卷挂载配置。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-docker-multistage-build | 提供可复现的根目录多阶段 Dockerfile 与 .dockerignore，镜像内包含 filehub-server release 二进制与 admin-web dist | Dockerfile/.dockerignore；构建阶段不牵动业务源码 | 以锁文件与固定基础镜像换取可复现构建；构建时间较长 | `docker build -t filehub:dev .` 成功（如本环境无 Docker，则以 server release 构建 + admin-web dist 构建 + Dockerfile 静态校验作为定向依据） | 不做镜像仓库 CI/推送 |
| P-002 | fh-docker-web-proxy | nginx 在 80 端口提供管理页，`/account/` 与 `/api/v1/` 反代到容器内 server，SPA 路由回退 | docker/nginx.conf；不修改 admin-web 路由与 API 契约 | 同源访问消除 CORS；反代层可后续再挂 TLS | 容器启动后 `GET /` 返回 admin-web index，`POST /account/login` 与 `/api/v1/projects` 经反代可达 | 不在镜像内终结 HTTPS |
| P-003 | fh-docker-runtime-config | 入口脚本按环境变量生成 server 配置：数据目录固定 `/data`（不提供数据目录环境变量）、`FH_SESSION_KEY`（缺省自动生成并持久化）、管理员账号、`FH_SERVER_PORT`、`FH_MAX_ARCHIVE_BYTES` | docker/entrypoint.sh；不改 server 配置 schema | 固定容器内 `/data`，外部位置由 `-v <host-path>:/data` 决定；缺省会话密钥自动持久化换取重启可用性 | `docker run -v $(mktemp -d):/data ...` 启动后 `/data/filehub.db` 与 `/data/files` 生成，数据库路径与文件路径固定指向 `/data` | 不支持多管理员 env 配置 |
| P-004 | fh-docker-docs | `docker/README.md` 说明构建、运行、卷挂载与 env 清单，更新变更记录与完成报告 | docker/README.md + docs/changes/018-docker-all-in-one.md | 以中文文档固化运行示例与默认凭据告警 | README 中的 docker run 示例可直接复制运行 | 不写独立部署运维手册 |

## Success Criteria

- 用户可见结果：
  - `docker build -t filehub:dev .` 成功，`docker run -p 8080:80 -v <data-dir>:/data \
    -e FH_SESSION_KEY=... -e FH_ADMIN_PASSWORD=... filehub:dev` 后，浏览器访问
    `http://127.0.0.1:8080` 能打开管理页并完成登录；
  - 数据目录中生成 `filehub.db` 与 `files/`，固定位于挂载卷所对应的容器 `/data`
    目录；重建容器挂载同一卷后数据仍在；
  - `docker run --help`/README 明确列出全部环境变量与默认值。
- 必需证据（按本环境能力取可执行集）：
  - `cargo build --release -p filehub-server` 通过；
  - `cd admin-web && VITE_API_BASE_URL=/ npm run build` 通过且 dist 可被 nginx 使用；
  - `sh -n docker/entrypoint.sh` 通过；nginx 站点配置按 nginx 语法复核；
  - 若环境提供 Docker 守护进程：真实 `docker build` + 容器启动冒烟，并记录结果。
- 明确非目标：不实现 HTTPS、多架构、镜像推送、自动备份；不改 server/admin-web
  默认行为。

## Risks

- 默认凭据暴露（中）：`FH_ADMIN_PASSWORD` 缺省为 `change-me` 且会话密钥自动生成，
  README/日志会告警；生产使用必须显式设置密码与 `FH_SESSION_KEY`。
- 进程/生命周期管理（低）：入口脚本同时拉起 nginx 与 server，任一进程退出时容器
  退出并由 Docker 重启策略接管；不做进程内自愈守护。
- 数据目录权限（低）：容器默认以 root 启动以保证 `-v` 挂载卷可写；README 记录以
  `--user` 运行时的数据目录属主要求。
- 构建可复现性（低）：Rust 与 Node 阶段依赖 Cargo.lock/package-lock；基础镜像 tag
  锁定大版本，registry 层版本漂移由镜像 tag 管理。
