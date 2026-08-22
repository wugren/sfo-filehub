---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-21
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认（按提案显示内容、standard 层级执行，含 loopback HTTP 降级边界）

# filehub-cli server 地址按 Docker 语义重构（无协议头 + HTTPS 优先/HTTP 降级）

Risk profile: not-created（standard 层级不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-21 当前用户回复「确认」，按提案显示内容以 standard 层级
    执行；进入 lower-tier 交付流（pre-edit 基线 -> 变更记录 -> 实现与验证 ->
    独立缺陷发现 -> 完成报告）。
- Tier rationale / triggered boundaries:
  - 修改 CLI 的 server 地址语义、本地凭据 key 与 HTTP 传输候选顺序，涉及公共
    命令行使用方式与既有配置文件兼容，普通 bugfix/refactor 之外还触碰 CLI 公共
    行为边界，不满足 trivial 的「无公共契约/CLI 影响」条件；
  - 变化有界且集中在 `cli` 模块，不改服务端、数据库、凭据明文格式、依赖图与
    部署面，不构成 material consequence，不触发 high-risk；
  - 按仓库默认，有界单项目功能/重构工作走 standard：变更记录 + 定向验证 +
    独立缺陷发现完成后报告。
- Proposal and tier confirmation:
  - 本提案需当前用户明确确认后才能执行。用户可选择按此提案确认（standard）、
    以替换层级（trivial/standard/high-risk）确认，或要求修订提案；
  - 确认后将 `workflow_tier` 从 `pending` 写为确认层级并把本提案置为
    `status: approved`。

## Background and Goal

当前现象：同一台本地服务已用 `filehub login http://127.0.0.1:8080` 登录，随后
执行 `filehub publish ... 127.0.0.1:8080` 报 `https://127.0.0.1:8080 未登录`。
根因是 server 地址被当作「凭据 key + 传输地址」两用：无协议时默认补 `https://`，
于是 `http://127.0.0.1:8080` 与 `127.0.0.1:8080` 被看成两个服务器。

Docker 的模型是把两者分开：凭据身份是 `host[:port]`（不带协议），实际请求总是
「HTTPS 优先」，只有 registry 被判定为不安全（loopback 默认即不安全）时才把
HTTP 作为降级候选。

目标：filehub-cli 的 server 参数不再要求协议头；登录、发布、下载等命令以
`host[:port]` 作为凭据身份匹配；请求层按 HTTPS 优先、可降级 HTTP 的候选端点
执行，并兼容既有带协议头的本地配置。

## Scope

### In scope

1. server 地址统一归一化为身份 `host[:port]`：无协议、显式 `http://`、
   `https://`、尾随 `/` 均落到同一身份；不再以协议头区分服务器；
2. 本地凭据存储与查找改用身份 key；对旧配置中已存在的带协议 key 提供
   「剥协议/路径后比较」的兼容匹配（等价 Docker 的 ConvertToHostname 语义），
   已登录用户无需重新 login；
3. 传输层实现候选端点：先 `https://host[:port]`；对 loopback 身份
   （`localhost`、`127.0.0.0/8`、`::1`）在连接/传输失败时降级尝试
   `http://host[:port]`，非 loopback 且无配置的服务器只走 HTTPS；
4. 同步更新 apiclient、credential_store、命令层调用点与相应单元/集成测试；
   补充「无协议登录 + 本地 HTTP 服务可用」「旧 http 凭据无协议复用」等正反例。

### Out of scope

- 不新增 insecure-registries 配置清单；本次仅实现 Docker 默认的 loopback
  降级，非 loopback 明文 HTTP 不开放，避免扩大安全面；
- 不修改服务端代码、服务端 TLS/HTTPS 终结配置、`docs/api/v1-contract.md`；
- 不加新命令行参数、不改退出码、不改 token/session 凭据互斥与保存格式；
- 不改 admin-web 前端；不迁移/重写既有配置文件（仅读取时兼容查找）。

### Boundary with neighboring modules

仅改 `cli` 模块（`cli/src/apiclient/`、`cli/src/credential_store/`、
`cli/src/cli/` 与 `cli/tests/`）及一份 task 变更记录；保留 `filehub-server`
与既有配置格式不变。

## Requirement Review

需求合理：Docker 的「身份与传输分离」语义已被广泛验证，能直接解决当前
`http://` 登录、无协议发布互相认不出的问题，同时保持本地开发默认可用。

选择的方向：
- 凭据匹配采用身份 key + 旧 key 兼容查找，避免破坏已登录用户的 config.toml；
- 传输只增加 loopback HTTP 降级，把安全面收缩到 Docker 默认范围；
- 显式协议头仍接受但仅作为历史输入兼容，不改变身份与候选顺序语义（与 Docker
  实际解析行为一致：SERVER 中的协议只用于解析 host，最终走端点候选列表）。

主要代价：
- 本地 HTTP 服务的首次 HTTPS 尝试会失败一次再降级，产生一条连接失败的瞬态
  日志/延迟；这是实现 Docker 语义的必要代价，后续可考虑缓存端点偏好；
- 旧配置中含协议 key 的记录不会被原地重写为身份 key，读取时需做一次兼容查找；
  保持只读兼容、不写迁移，风险最低。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-server-identity | server 参数与本地凭据以 `host[:port]` 为身份；无协议/显式协议/尾随斜杠统一；旧带协议 key 兼容匹配 | credential_store + server 解析；不迁移文件 | 接受历史协议头输入换取兼容，不改用户文件 | 单元测试覆盖身份归一化与旧 key 查找；用「http 登录 + 无协议 publish」真实/模拟联调成功 | 不重写旧配置、不新增参数 |
| P-002 | fh-cli-https-first-fallback | 请求层 HTTPS 优先，loopback 在传输失败时降级 HTTP；非 loopback 仅 HTTPS | apiclient/命令层候选端点；不动服务端 | 本地 HTTP 多一次失败尝试 | integration 断言无协议 server 对本地 HTTP mock 的登录/发布成功；非 loopback 无 HTTP 候选 | 不开放任意明文 HTTP |
| P-003 | fh-cli-identity-regression | 全部 CLI 命令（login/publish/download/versions/new-version/lock/delete-app）沿用同一语义并回归 | cli 命令层与测试 | 一次统一改造避免命令间语义分叉 | `cargo test -p filehub-cli` 全量通过；既有测试按新语义更新后通过 | 不逐命令打补丁 |

## Success Criteria

- 用户可见结果：`filehub login http://127.0.0.1:8080` 后，`filehub publish
  <path> <project>:<version> 127.0.0.1:8080` 直接成功，不再提示未登录；
  `filehub login 127.0.0.1:8080` 也能对本地 HTTP 服务正常登录并保存
  `127.0.0.1:8080` 身份凭据。
- 必需证据：身份归一化/旧 key 兼容、HTTPS 优先与 loopback HTTP 降级的单元 +
  集成测试通过；`cargo test -p filehub-cli` 全量通过；变更记录与完成报告
  记录行为变化与验证。
- 明确非目标：不做 insecure-registries 配置、不改服务端、不改配置迁移工具。

## Risks

- 既有配置兼容（中）：旧 key 是 `http://127.0.0.1:8080` 形式，新查找必须能
  命中；本提案以只读兼容查找覆盖，并写入测试。
- 传输语义变化（低）：无协议地址从直接 `https://` 变为 HTTPS 先试、loopback
  降级 HTTP；对已部署的 HTTPS 服务无行为回退，对本地 HTTP 服务多一次失败尝试。
- 安全边界（低）：HTTP 降级严格限制 loopback，避免明文凭据跨越网络边界；
  非 loopback 若要 HTTP 需后续单独评估配置面，不在本任务范围。
