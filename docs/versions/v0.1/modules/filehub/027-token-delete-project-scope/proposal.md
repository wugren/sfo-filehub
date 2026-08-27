---
task_manifest: task.yaml
status: approved
---

# Token 删除项目补项目级校验 + 空项目集合语义

Risk profile: not-created（standard 层级不创建 risk-profile）

## Approval Record

- approver: user
- approval_date: 2026-08-23
- user_statement: 用户 2026-08-23 回复「确认，按standard任务完成就好」，确认
  采纳提案（token delete 完整项目级校验 + 空项目集合 = All），并选择 standard
  层级；默认的 administration scope 要求未被用户修正，按默认方向执行。

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 修改的是授权/删除语义：token 删除项目将从「仅 Feature 级 projects:delete +
    账号 Owner」收紧为「项目级校验（project_scope + administration scope +
    用户项目管理员权限）」，同时把空项目集合从报错改为 All 语义；
  - 命中安全/授权边界与既有行为语义变更（存量 token 若只有 projects:delete
    scope 将不能再删除项目），按 high-risk 全流程执行；若用户选择标准/轻量
    层级，将把剩余风险显式记录在变更与验收中。
- Proposal and tier confirmation: 用户 2026-08-23 回复「确认，按standard任务
  完成就好」——确认提案并选择 standard 层级；standard 层级跳过设计/测试/验收
  分期文档与风险档案，剩余风险（授权语义变更、存量 token 行为变化）记录在
  change record 与 completion-report 中显式标注。

## Background and Goal

- 背景：025 任务验收（acceptance-report.md 的后续任务建议）记录了两处存量
  边界：`projects.service.rs::delete` 只校验
  `Resource::Feature(ProjectsDelete)`（账号 Owner + scope），token 不做
  项目级/项目范围校验；`ProjectScope::Specified(vec![])` 目前会被
  `FromStr` 拒绝或落库为空串，语义未定义。
- 当前用户明确要求：
  1. 为 token 的 delete 补项目级校验；
  2. 空的项目集合表示可以操作 token 所属用户的所有项目。
- 目标：token 删除项目必须同时通过项目级授权判定；`Specified([])` 与
  `ProjectScope::All` 等价（仍受 token 所属用户自身项目权限限制）。

## Scope

### In scope

- `server/src/projects/service.rs::delete`：对 `Principal::Token` 在执行
  Feature 级 projects:delete 校验后，追加项目级校验——复用
  `checker.can_access(actor, Resource::Project(project_id), ACTION_ADMIN)`，
  即依次要求 token project_scope 包含目标项目、token 具 `administration`
  scope、token 所属用户为目标项目 owner 或被授予 admin；项目不存在时保持
  not_found、无权限返回 forbidden（与 set_visibility 语义一致）；
- `server/src/model/scope.rs`：新增空集合归一化语义——`Specified(vec![])`
  等价于 `All`；`ProjectScope::from_str` 对空串/空集合返回 `All` 而不是
  报错；
- `server/src/tokens/service.rs`：create/update 在落库前把空
  `Specified` 归一化为 `All`（rotate/resolve 复用既有存储值）；
- 更新 `docs/api/v1-contract.md` 的 project_scope 语义说明（空集合 = 全部
  项目 = 受所属用户权限限制）；
- 回归测试：token 删除在范围外/无 admin 权限/无 administration scope 时
  拒绝，范围内且用户为 admin 时放行；空 Specified 创建/更新后 resolve 返回
  All。

### Out of scope

- 不修改 User session 的 delete 语义（仍为 Feature 级 projects:delete +
  账号 Owner）；
- 不修改 checker 其他动作（metadata/artifacts/administration）的判定；
- 不修改 token 管理 API 请求/响应 JSON 形状；
- 不做数据库 schema 迁移（project_scope 列与存储格式不变：空输入归一化为
  `"all"` 落库）；
- 不改管理端 UI / CLI 行为。

### Boundary with neighboring modules

- 权限判定仍收敛在 `permissions::checker`，projects 服务只追加调用；
- token 权限语义沿用 025 的数据库权威链路（JWT 不含权限属性），本任务不
  改签发/解析。

## Requirement Review

- 需求合理：delete 是项目变更操作，理应与 set_visibility 一样受项目级授权
  约束；空集合 = All 是常见且直观的语义，与「受所属用户权限限制」的既有
  All 行为一致。
- 关键权衡：
  1. 删除的 token 除 projects:delete scope 外还需 `administration` scope，
     这是「项目级校验」与 Actions 语义一致的必然结果；存量仅有
     projects:delete 的 token 删除会被 403，如需保持轻量的项目范围校验
     可选方案是只查 project_scope 不查 administration scope（默认按完整
     项目级校验执行）；
  2. 空集合在 create/update 落库前归一化为 `All`，并在 FromStr 兼容空值，
     避免旧数据/手写 DB 出现空串无法解析；
  3. “所有项目”仍以所属用户项目权限为界（All 语义），不会放大为越权。
- 选择方向：最小闭环——delete 复用 checker Project 分支
  （project_scope -> scope -> 用户项目权限 fail-closed），空集合在模型层
  归一化。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-token-delete-project-gate | token 删除项目前追加项目级校验（checker Project + ACTION_ADMIN），项目不存在返回 404、无权限 403；User session 路径不变 | 仅 token principal 生效；不改变 Feature 级校验顺序 | token 需同时具 projects:delete 与 administration scope；换取与 set_visibility 一致的授权边界 | 单元测试：范围外/无 admin/无 administration scope 均拒绝，范围内用户 admin 放行 | 不改 User delete 语义 |
| P-002 | fh-token-empty-scope-all | `Specified([])` 与 `All` 等价：FromStr 空值返回 All，create/update 落库前归一化为 All | 不改变非空 Specified 的既有包含语义 | 用归一化替代报错，兼容存量空串 | create/update 后 resolve.project_scope == All；DB 存 "all" | 不改 token API JSON 形状 |
| P-003 | fh-token-delete-empty-tests | 新增/更新单元回归测试与 API 契约文档说明 | 测试仅在 server 单元层；契约文档仅补两处语义说明 | 最小测试集覆盖删除矩阵与空集合两条路径 | `cargo test -p filehub-server` 通过，新增断言全部命中 | 不新增 UI/CLI 测试 |

## Success Criteria

- 系统可见结果：
  - token 删除范围外项目 403，删除自己无管理员权限的项目 403，删除不存在
    项目 404，范围内且用户为 admin 可删除；
  - 创建/更新提交 `{"Specified": []}` 后，token 的 project_scope 为 All，
    DB 存 `"all"`，旧空串数据按 All 解析；
  - User session 删除路径行为不变。
- 必需证据：`cargo test -p filehub-server`（或统一 test-run 入口）全绿，
  含新增删除矩阵与空集合断言；验收记录反例搜索（越权删除、空集合绕过）。
- 显式非目标：HTTP JSON 形状、schema、管理端/CLI 均不变。

## Risks

- 存量 token 行为变化：只有 projects:delete 而无 administration scope 的
  token 删除将 403；缓解：变更记录与验收中显式标注，管理端表单本就可勾选
  administration scope。
- 空集合语义放宽 ≠ 越权：All 仍受 checker 用户项目权限限制，不会放大到
  token 所属用户无权操作的项目。
- 回归风险：删除路径涉及 projects/versions 引用，删除后引用不可见语义不
  变；验收做失败路径反例。
