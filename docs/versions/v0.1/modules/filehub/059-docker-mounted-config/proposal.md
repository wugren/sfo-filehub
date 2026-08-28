---
task_manifest: task.yaml
status: approved
---

# Docker 镜像改为读取挂载的服务端 YAML 配置

Risk profile: not-created (replace with ./risk-profile.yaml only after high-risk confirmation)

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 本任务改变正式 Docker 镜像的配置入口和启动前置条件：现有 `FH_*` 环境变量生成配置的部署将不再兼容，升级前必须准备并挂载 YAML；
  - 配置包含管理员凭据、Ed25519 会话私钥、数据路径与监听端口，直接涉及 security、production configuration、release/deployment、compatibility/rollback 边界；
  - CI 容器冒烟也必须切换到挂载配置，且需要验证缺失配置失败关闭、只读挂载不被覆盖、nginx 与 server 固定内部端口一致，因此建议 `high-risk`。
- Proposal and tier confirmation: 2026-08-28 用户回复「确认，trivial」，批准本提案并明确选择 `trivial` 层级。该选择低于建议的 `high-risk`；执行保留部署兼容性、安全配置、失败关闭和 CI 冒烟的定向检查，并在完成报告中记录真实容器环境不可用时的残余风险。

## Approval Record

- approver: 用户
- approval_date: 2026-08-28
- user_statement: 「确认，trivial」
- selected_tier: trivial

## Background and Goal

当前 Docker 入口脚本读取 `FH_ADMIN_PASSWORD`、`FH_SESSION_PRIVATE_KEY`、`FH_SERVER_PORT` 等环境变量，每次启动都生成 `/etc/filehub/filehub-server.yaml`，然后用该文件启动 server。这使容器部署方式与服务端已经确立的 YAML 配置契约不一致，也无法安全地把一份只读配置挂载后原样使用。

目标是让 Docker 镜像只读取用户提供的服务端 YAML：用户在宿主机准备 `filehub-server.yaml`，以只读方式挂载到固定路径，容器不再从环境变量拼装、覆盖或补全配置。

## Scope

### In scope

- 固定容器配置路径为 `/etc/filehub/filehub-server.yaml`；入口脚本检查文件存在、是普通文件且可读，缺失或不可读时给出明确错误并失败退出；
- 删除 Docker 入口中的全部 `FH_*` 配置读取、YAML 生成、默认管理员密码和私钥自动生成逻辑；`filehub-server` 通过位置参数读取固定配置文件；
- 保持 `/data` 为持久化数据根，提供 Docker 专用 `docker/filehub-server.example.yaml`，明确设置 `db_path: /data/filehub.db`、`files.data_dir: /data/files`、`server.server_addr: 127.0.0.1`、`server.port: 8080`；
- 将容器内 server 端口 `8080` 定为镜像内部契约，nginx 固定反代 `127.0.0.1:8080`；外部访问端口继续由 Docker `-p <host-port>:80` 决定；配置若修改 server 端口不属于受支持的镜像配置；
- Dockerfile 不再安装仅用于生成配置/私钥的 `jq`、`openssl`，nginx 配置作为最终配置直接复制，不再由入口脚本替换占位符；
- 更新根 README 与 `docker/README.md`，给出宿主配置复制、私钥生成/保护、只读挂载、数据卷挂载及升级迁移示例；删除 Docker `FH_*` 环境变量说明；
- 更新 GitHub Actions 镜像冒烟，使其挂载 Docker 示例 YAML，不再注入 `FH_ADMIN_PASSWORD`；
- 新增 Docker 配置契约测试，并扩展现有 Actions 契约测试，覆盖固定路径、无环境变量生成、只读配置不改写、固定内部端口、缺失配置失败和 CI 冒烟挂载。

### Out of scope

- 不删除非 Docker 场景下 `filehub-server` 自身现有的位置参数/`FH_CONFIG` 兼容入口；Docker 入口始终显式传入固定配置路径，不使用该环境变量；
- 不改变服务端 YAML schema、账号同步、Ed25519 算法、数据库 schema、API 或数据目录内容；
- 不支持通过环境变量覆盖 YAML 的单个字段，也不支持模板替换、配置合并或 secret 插值；
- 不在镜像内自动生成管理员密码或 Ed25519 私钥；用户必须在部署前写入配置，或由外部配置/secret 管理系统生成并挂载完整 YAML；
- 不增加 YAML 解析工具以动态生成 nginx 配置；容器内 server 端口固定为 `8080`；
- 不触碰工作树中已有的 058 `latest` 发布改动、`Cargo.lock`、`harness/scripts/edit-guard.py`、根目录 `filehub-server.yaml`、`filehub.db` 等无关修改。

### Boundary with neighboring modules

本任务改变 Docker 镜像和 CI 冒烟的部署入口，不改变 server 的配置数据模型或普通二进制启动方式。宿主机持久化数据仍只位于 `/data`；挂载到 `/etc/filehub/filehub-server.yaml` 的配置属于独立部署资产，不随数据卷自动生成。

## Requirement Review

需求合理，且与服务端 YAML-only 配置契约一致。采用只读挂载可以让配置内容可审计、可版本化管理，并避免入口脚本把环境变量静默转换成另一份实际生效配置。

主要代价是一次有意的部署兼容性中断：旧的 `-e FH_*` 启动命令将失败，因为镜像要求显式挂载 YAML。配置中包含密码或密码哈希以及完整 Ed25519 私钥，宿主文件必须按 secret 管理，建议权限 `0600` 并使用只读 bind mount。旧部署若希望保留现有登录会话，需要在升级前从数据卷读取 `/data/.session_private_key.pem`，把同一私钥安全写入新 YAML；换新私钥会让已有 session/refresh JWT 失效并要求重新登录。

为避免在 shell 中用不完整规则解析 YAML，容器内 server 端口固定为 `8080`，由 server 自己负责完整 YAML 解析与字段校验；nginx 直接反代该固定端口。用户仍可自由改变宿主机暴露端口，例如 `-p 9000:80`。

回滚方式是恢复上一版本镜像及其原有 `FH_*` 参数；`/data/filehub.db` 与 `/data/files` 布局不变。新 YAML 应在切换镜像前准备并验证，避免部署时因缺失配置停机。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-docker-mounted-config | Docker 镜像仅从只读挂载的 `/etc/filehub/filehub-server.yaml` 启动，不读取 `FH_*` 生成配置；提供 Docker 专用配置模板、固定内部 8080 端口并同步 CI/文档/契约测试 | Docker 打包、入口、nginx、文档、CI 冒烟与静态测试；server YAML schema 不变 | 部署更显式可审计，但旧环境变量启动命令不兼容，用户必须自行保护配置中的密码和私钥 | Docker 契约测试证明无 `FH_*`/jq/openssl 生成路径、缺失配置失败、配置未改写、nginx/server 8080 对齐；Actions 契约证明 smoke 只读挂载示例配置；YAML/Shell/工作流检查通过；有 Docker 时完成真实启动与缺失配置反例 | 不支持字段级环境变量覆盖、自动密钥生成、动态内部端口、配置合并或 server schema 修改 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 用户执行 `docker run ... --mount type=bind,src=/host/filehub-server.yaml,dst=/etc/filehub/filehub-server.yaml,readonly ...` 后，镜像原样读取该 YAML 启动 server 与 nginx；
  - 未挂载配置、路径错误或配置不可读/非法时容器失败退出，不生成默认管理员密码、私钥或替代配置；
  - `docker run -e FH_ADMIN_PASSWORD=...` 等旧环境变量不再改变服务端配置；
  - `/data/filehub.db` 与 `/data/files` 继续通过数据卷持久化。
- Required evidence:
  - Docker 配置契约测试、GitHub Actions unit/DV/integration 契约测试通过；
  - `sh -n docker/entrypoint.sh`、YAML 解析、nginx 配置静态检查、任务范围 `git diff --check` 通过；
  - 若本机存在 Docker daemon，验证合法只读配置启动成功、未挂载配置失败、配置文件哈希启动前后不变；否则明确保留 hosted runner 验证缺口。
- Explicit non-goals:
  - 不修改应用 API、数据 schema、server 配置 schema，不保留 Docker `FH_*` 兼容层。

## Risks

- 这是 breaking deployment change：依赖 `FH_*` 的现有 Compose/Kubernetes/CLI 启动配置必须迁移为完整 YAML 挂载。
- YAML 同时承载管理员凭据与会话签名私钥；错误的文件权限、提交到源码仓库或泄露挂载内容会扩大安全风险。
- 更换/丢失 Ed25519 私钥会使已有 session/refresh JWT 失效；必须显式迁移旧私钥或接受重新登录。
- 内部 server 端口固定为 8080；用户若在 YAML 中改成其他值，server 可能启动但 nginx 无法连通，因此示例、文档和契约测试必须突出该限制。
- 本地若无 Docker daemon，不能把静态检查宣称为真实容器启动验证；最终线上行为仍需 hosted runner 镜像冒烟证明。
