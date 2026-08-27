---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-24
user_statement: 用户 2026-08-24 回复「确认，自动完成」，确认采纳提案并按
  high-risk 层级自动执行完整分层流程。
---

# Token 属性修改不再自动重签：显式「重新签发」按钮

Risk profile: ./risk-profile.yaml

## Approval Record

- approver: user
- approval_date: 2026-08-24
- user_statement: 用户 2026-08-24 回复「确认，自动完成」，确认采纳提案，
  并按 high-risk 全流程自动执行（design -> implementation -> testing ->
  acceptance），不另行逐阶段征询。

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 本任务改变 token 授权/吊销语义：属性修改（name/scopes/project_scope）
    不再重签，已签发的旧 JWT 副本在权限变更后继续有效、权限立即按数据库
    生效；只有显式「重新签发」才更换验签公钥使旧 JWT 失效——这是对既有
    「权限变更即重签换钥」行为的一次授权语义变更；
  - 同时变更公开 HTTP 契约形状：`POST /api/v1/tokens/{id}` 从
    `TokenIssued`（重签）| `TokenSummary` 变为始终返回 `TokenSummary`，
    `TokenUpdateRequest` 移除 `expires_at`；
  - 按任务入口规则，安全/信任边界与公开契约变更默认应走 high-risk
    全流程；若用户选择 standard/trivial 轻量层级，剩余风险（旧 JWT 在权限
    收紧后仍有效的语义变化、重签默认不过期的取舍）会显式记录在变更与验收中。
- Proposal and tier confirmation: 用户 2026-08-24 回复「确认，自动完成」——
  确认采纳提案并选择 high-risk 全流程；重签默认不过期、属性修改不使旧 JWT
  失效两项取舍按提案原文执行，未提出修订。

## Background and Goal

- 现象（评审第 3 项，高）：修改 Token 权限会意外移除过期时间。
  - `server/src/tokens/model.rs` `TokenUpdateRequest.expires_at` 注释规定
    `None = 不修改`；
  - `server/src/tokens/service.rs::update` 把 `project_scope.is_some() ||
    scopes.is_some() || expires_at.is_some()` 当作必须重签的条件；
  - 重签时 `patch.expires_at.unwrap_or(None)` 把缺省值转成 `None`，签发
    不带 `exp` 的新 JWT——原本有限期的 Token 因此变成永久 Token。
- 用户明确要求：修改 token 属性时可以不要重签；重签应通过显式按钮触发，
  只有点击按钮时才重新签发。

## Scope

### In scope

- `server/src/tokens/model.rs`：`TokenUpdateRequest` 移除 `expires_at`
  （属性修改不再触碰过期时间，不再伴随 JWT 签发）；
- `server/src/tokens/service.rs` / `mod.rs` / `http.rs`：`update()` 只落库
  属性变更（name/project_scope 归一化/scopes 原子替换），不再生成密钥对、
  不再签发 JWT，返回 `TokenSummary`；旧 JWT 副本继续有效，权限变更通过
  `resolve` 数据库权威立即生效；
- 显式重签：继续使用既有 `POST /api/v1/tokens/{id}/rotate`（换新验签公钥
  + 一次性展示新 JWT + 旧 JWT 立即失效），在管理端把该操作显式呈现为
  「重新签发」按钮（沿用现有 rotate 语义：重签 JWT 默认不过期；后续如需
  在重签时选择有效期可另立提案）；
- `admin-web`：编辑弹窗移除有效期选择与「修改即重签」警告，改为提示
  「属性修改仅保存、不重新签发；需要新 JWT 请点『重新签发』」；列表行
  「轮换」按钮改名为「重新签发」并更新确认文案；`updateToken` 返回类型
  收敛为 `TokenSummary`，保存不再弹 JWT；
- `docs/api/v1-contract.md`：更新 tokens 路由语义与 DTO 形状（属性修改
  不重签；重签=rotate）；
- 测试：更新 server unit 与 admin-web 单测/契约测试，并新增回归：
  属性修改返回 summary 且不产生新 JWT、旧 JWT 的 `exp` 保持原样、
  权限变更后旧 JWT 仍可 resolve 且权限按数据库生效、rotate 使旧 JWT 失效。

### Out of scope

- 不修改 create / list / revoke / resolve 行为；
- 不新增 `expires_at` 数据库字段或迁移（过期时间仍只由 JWT exp 承载，
  重签无法沿用旧期限，由显式重签流程承担该取舍）；
- 不新增 `/resign` 端点（复用 `/rotate` 语义，仅前端展示为重签）；
- 不改 CLI（CLI 未使用 token update/rotate）；
- 不引入 `expires_at: null` 清除语义。

### Boundary with neighboring modules

- 权限判定仍收敛在 `permissions::checker` 与 `tokens::resolve` 数据库
  权威设计（025 已确立、027 沿用），本任务不重新引入 JWT claims 权限属性；
- 撤销/重签的并发与事务边界沿用现有实现。

## Requirement Review

- 用户要求合理：在 JWT 不携带权限属性、resolve 以数据库为权威的前提下，
  属性修改天然不需要重签；自动重签既破坏 exp，又让「仅改名/调权限」产生
  一次不必要的无效化副作用，与 Git 风格 token 录入模型不符。
- 关键取舍：属性修改不再使旧 JWT 副本失效。权限收紧/放宽都立即按数据库
  生效；需要让旧副本失效时，用户点击显式「重新签发」。这是用户指定的
  方向，与 025 的数据库权威设计一致。
- 另一取舍：重签沿用 rotate 的「默认不过期」语义（服务端未存旧 exp，
  无法自动沿用）。该操作现在完全显式，确认弹窗会说明新 JWT 不过期、
  旧 JWT 立即失效。若用户希望在重签对话框中选择有效期，可在确认时提出
  修订。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-token-update-no-resign | 属性修改（name/project_scope/scopes）只落库并返回 TokenSummary，不生成密钥、不签发 JWT | 修改服务端 update 路径与 HTTP/DTO 契约 | 属性修改不再使旧 JWT 副本失效；权限变更立即按数据库生效 | update 返回 TokenSummary；resolve 旧 JWT 正常且权限/exp 符合预期 | 不新增数据库字段、不新增 /resign 端点 |
| P-002 | fh-token-explicit-resign-action | 管理端提供显式「重新签发」操作，仅点击该按钮才重签并一次性展示新 JWT | 修改 admin-web 编辑表单与行操作按钮 | 重签沿用 rotate 默认不过期语义；确认弹窗明示旧 JWT 立即失效 | 列表行「重新签发」按钮、编辑弹窗移除有效期/重签警告且文案一致 | 不改变 rotate 端点语义、不新增 UI 抽象 |
| P-003 | fh-token-no-resign-regression-tests | server 与 admin-web 测试更新 + 回归断言 | 修改 server/tests/unit/tokens.rs 与 admin-web tests | 断言 exp 不被破坏、update 不产出 JWT、权限按 DB 生效、rotate 使旧 JWT 失效 | 全套 server 与 admin-web 测试通过 | 不新增测试框架/基础设施 |

## Success Criteria

- 属性修改请求不再返回/签发新 JWT；旧 JWT 的 `exp` 保持原样，权限变更后
  resolve 立即按数据库生效；
- 管理端存在显式「重新签发」入口，仅点击该按钮才重签，重签后新 JWT
  一次性展示、旧 JWT 立即失效；
- `cargo test -p filehub-server`（unit + dv + integration）与
  `admin-web` build/unit/integration 全部通过；
- `docs/api/v1-contract.md` 与实现一致。

## Risks

- 授权/吊销语义变更：权限修改不再自动使旧 JWT 副本失效——与用户显式
  指令一致，但在验收中需显式确认「收紧权限后旧副本仍可访问」的边界成立；
- 公开契约变更：update 响应/DTO 形状变化，需要前后端同步收敛并更新契约
  文档；
- 重签默认不过期：显式操作 + 确认文案兜底，任何有限期 Token 不再被
  隐式重签转永久。
