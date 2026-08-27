---
task_manifest: task.yaml
status: approved
---

# Token 限定项目范围（project_scope）实际生效修复

Risk profile: ./risk-profile.yaml

## Approval Record

- approver: user
- approval_date: 2026-08-23
- user_statement: 用户 2026-08-23 回复「确认，自动完成」，确认采纳修订后提案
  （权限全部不放入 JWT、服务端数据库判定、不考虑旧 JWT 兼容）并按 high-risk
  层级自动执行完整流程。

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 本任务是授权边界修复：当前「指定项目」Token 可以访问其所属用户有权访问的
    全部项目，超出 project_scope 限定，属于实质性安全/权限边界缺陷；
  - 修复修改权限判定路径（resolve 构造 + checker 判定）与
    `Principal::Token` 服务端内部模型，命中安全/授权边界风险类别；JWT claims、
    API JSON 形状与数据库 schema 均不变，无契约/兼容性风险；
  - 因此按 high-risk 的 proposal -> design -> implementation -> testing ->
    acceptance 全流程执行；若用户选择标准/轻量层级，会跳过完整分期流程，
    剩余风险（授权边界回归、旧 JWT 兼容）将留在实现与验收记录中显式标注。
- Proposal and tier confirmation: 用户 2026-08-23 回复「确认，自动完成」，
  原样确认提案并确认 high-risk 层级。

## Background and Goal

- 现象：Token 创建/更新时提交的 `project_scope` 只写入数据库，权限链路完全
  没有使用它。问题证据：
  - `server/src/tokens/model.rs:33`：`TokenPayload`（写入 JWT 的载荷）携带
    `scopes` 权限集合；按当前用户要求，权限属性应整体从 JWT 移除、由服务端
    数据库判定；
  - `server/src/tokens/service.rs:158`：create 计算了
    `project_scope = req.project_scope.unwrap_or(ProjectScope::All)`，
    但组装 payload 时未放入；update（~243 行）与 rotate（~288 行）同样丢弃；
  - `server/src/tokens/service.rs:323`：resolve 只查 `owner_id/public_key_pem/
    revoked_at`，没有读出 `project_scope`，且 scopes 直接信 JWT claims
    （`claims.data.scopes`）而不是数据库 `token_scopes`；
  - `server/src/http/auth.rs:34`：`TokenPrincipal -> Principal::Token` 未携带
    project_scope；
  - `server/src/permissions/checker.rs:133`：`Resource::Project` 分支对
    `Principal::Token` 只做 scope 交集 + 用户项目权限，未校验
    project_scope 是否包含目标项目。
- 后果：限定为「指定项目」的 Token 实际可按其所属用户的权限访问所有项目，包括
  项目列表（`projects.list` 按 `can_access(metadata:read)` 过滤，同样被绕过）。
- 目标：让 `project_scope` 成为 token 授权的硬性第二限制：
  - 限定项目集合之外的项目一律拒绝（无论用户本身对该项目有何权限）；
  - 「全部项目」保持当前行为；
  - 修复后新旧 token 都被正确限制（旧 JWT 没有该 claim 也不能绕过）。

## Scope

### In scope

- 遵循当前用户明确要求：所有权限属性不放入 JWT claim，且不考虑旧 JWT 兼容
  （新增/移除字段不需要任何兼容特判或 serde 默认值）；
- `TokenPayload` 移除 `scopes` 字段，只保留 `token_id/user_id`；不新增
  `project_scope`；create/update/rotate 三个签发路径不再把 scopes 写入
  JWT 载荷，`token_scopes` 表的维护逻辑不变；
- `TokenPrincipal` 与 `Principal::Token` 新增 `project_scope` 字段，
  保留 `scopes` 字段，`http/auth.rs` 桥接透传；
- `TokenService::resolve` 改为以数据库 `tokens.project_scope` 为权威值构造
  `TokenPrincipal`，同时以数据库 `token_scopes` 表读出 scopes（不再信任
  JWT claims），权限判定全部在服务端完成；旧 JWT 中残留的 scopes/project_scope
  字段作为未知字段被忽略，不提供兼容读取逻辑；
- `SqlitePermissionChecker::can_access` 的 `Resource::Project` 分支对
  `Principal::Token` 增加项目范围校验（`Specified` 集合不包含目标项目即拒绝，
  然后才执行现有 scope 二次限制与用户项目权限判定）；
- 同步更新三类测试并新增回归用例：token 单元测试（create/update/rotate 后
  resolve 的 project_scope 正确、JWT claim 含 project_scope）、权限单元测试
  （指定项目 token 对限定项目放行、对外部项目拒绝、All 行为不变）与现有
  `Principal::Token` 构造点；
- 更新 `docs/api/v1-contract.md` 对 token JWT claims 的契约说明（如现有文档
  描述 claims），以及任务包内中文文档。

### Out of scope

- 不修改 token 管理 HTTP API 的请求/响应形状（`project_scope` 字段早已存在）；
- 不扩展 token JWT 的 claims 形状，且按当前用户要求收窄：JWT 不再携带
  scopes/project_scope 等权限属性；
- 不修改登录 session（`Principal::User`）的权限语义；
- 不修改 `Resource::Feature`（projects:create/delete）的判定——项目创建/删除
  没有目标项目，属于功能级 scope + 账号角色控制，不套用 project_scope；
- 不做数据库 schema 迁移（`tokens.project_scope` 列已存在）；
- 不新增管理端 UI 或 CLI 行为变化。

### Boundary with neighboring modules

- `permissions/checker.rs` 是唯一授权判定入口，`projects.list`、版本、下载等
  服务均通过 `can_access` 过滤，因此在本层修复即可覆盖所有项目资源消费方；
- `tokens/service.rs` 同时管理签发与解析，密钥轮换/重签语义保持现状
  （project_scope 变化即重签并更换验签公钥，旧 JWT 立即失效）。

## Requirement Review

- 需求合理且必须修复：project_scope 是 token 的核心安全属性之一
  （001 核心平台提案明确定义「带过期时间与项目级权限的 token 授权」），
  只落库不生效属于授权绕过缺陷。
- 关键设计权衡：
  1. 按当前用户要求，权限一律不放进 JWT：JWT 只承担凭据有效性（验签、
     sub/jti/iat/exp 与 token_id/user_id），scopes 与 project_scope 都由
     resolve 查库得到，权限判定完全在服务端完成；
  2. 权威值来源必然是数据库：resolve 本来就要查库取验签公钥与撤销状态，
     顺带读取 `token_scopes` 与 `project_scope` 成本极低，且不存在 claims
     与数据库双源不一致问题；用户明确表示不考虑兼容，不做旧 JWT 特判，
     旧载荷中的权限字段自然作为未知字段被忽略；
  3. 校验顺序：先验签并校验 claims 一致性，再从数据库读 scopes 与
     project_scope，最后在 checker 中做 `project_scope -> scope ->
     用户项目权限` 的三层判定，任何一层不满足即拒绝（fail closed）。
- 选择的方向：最小改动但完整闭环——scopes 与 project_scope 全部沿
  「DB -> TokenPrincipal -> Principal::Token -> checker」在服务端链路携带
  并判定，JWT claims 不再包含任何权限属性。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-token-permissions-server-side | TokenPayload 移除 scopes、不新增 project_scope（JWT 不含权限属性）；TokenPrincipal/Principal::Token 携带 scopes + project_scope；resolve 从 token_scopes / tokens 数据库读出并构造 Principal | 密钥生命周期与重签语义不变（scope/项目范围变更仍重签并更换验签公钥）；数据库为唯一权限权威；不做旧 JWT 兼容特判 | 用户明确要求「scopes 也去掉，不考虑兼容问题」；服务端查库换取单一权威、无双源漂移 | resolve 返回的 scopes/project_scope 与 DB 一致；JWT 解码后载荷不含 scopes/project_scope | 不扩展 JWT claims、不增加兼容层 |
| P-002 | fh-token-project-scope-enforce | checker 对 Token 访问 Project 资源时校验 project_scope 包含目标项目 | 仅 `Resource::Project` 生效；Feature 级别动作不套用 | 拒绝优先（fail closed）：范围外项目在 scope/用户权限之前直接拒绝 | 指定项目 token 对限定项目可访问、外部项目全部动作拒绝；All 行为不变 | 不改用户 session 语义 |
| P-003 | fh-token-project-scope-tests | 回归测试覆盖签发/解析/校验三层与「JWT 不含权限属性、resolve 纯查库」路径，更新 API 契约文档 | 测试只在 server 单元/集成层；不引入新依赖 | 用最小测试集覆盖绕过的两个关键路径（resolve 构造 + checker 判定） | `cargo test -p filehub-server`（或等价仓库测试）通过，新增断言全部命中 | 不新增 UI/CLI 测试 |

## Success Criteria

- 系统可见结果：
  - `resolve` 返回的 principal 携带与数据库一致的 project_scope；
  - `resolve` 返回的 scopes 来自 `token_scopes` 表（不再取自 JWT claims）；
  - 限定项目集合外的项目，token 的 metadata:read/artifacts:read/
    artifacts:write/administration 全部返回拒绝，`projects.list` 不再列出
    范围外项目；
  - 老 token 的 JWT 中若残留 scopes/project_scope 字段，会被忽略并按数据库
    值判定（用户明确要求不考虑兼容，不提供特判逻辑）。
- 必需证据：`cargo test -p filehub-server` 全绿（含新增回归断言）；任务包
  completion/acceptance 验收记录中列出反例搜索（外部项目放行、旧 claim
  缺失、update/rotate 路径）。
- 显式非目标：管理端/CLI 行为、API 请求/响应 JSON 形状、schema 均不变。

## Risks

- 授权边界回归：若 resolve 或 checker 任一层漏掉 project_scope，漏洞复现；
  缓解：三层测试 + 验收反例搜索，checker 采用 fail-closed 顺序。
- 双源漂移：JWT claims 不再包含任何权限属性，数据库是唯一权限来源，不存在
  claim 与存储值不一致的问题；变更记录中显式记录
  「权限不放入 JWT、服务端判定、不考虑旧 JWT 兼容」这一用户要求。
