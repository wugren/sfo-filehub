---
task_manifest: task.yaml
status: approved
---

# 登录后仍可读取无授权关系的 public 项目（User/Token 分支补齐 public 只读可见性）

Risk profile: not-created（标准层级不创建 risk-profile）

## Approval Record

- approver: user
- approval_date: 2026-08-24
- user_statement: 用户 2026-08-24 回复「确认」，采纳 standard 层级并批准本提案；
  token `project_scope` 维持 fail-closed（Specified 范围外即使 public 仍拒绝）。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 属于 server 权限核心的生产 bugfix，修改 `can_access` 的
    `Resource::Project` 用户/Token 分支运行行为，非 trivial（影响运行时代码，
    需要单元回归验证）；
  - 未触发 high-risk：修复只把「已公开（public）内容」的只读放行扩展到已登录
    User/Token，不涉及 private 数据、写动作、schema/迁移、公共契约、依赖或
    发布面；无 material 安全回退风险；
  - 可定向验证：权限单元测试 + 项目列表过滤断言。
- Proposal and tier confirmation: 用户 2026-08-24 回复「确认」，采纳 standard
  层级，本提案置为 `status: approved`；提案中列明的 token project_scope
  fail-closed 边界一并确认。

## Background and Goal

- 现象：Anonymous 可读取 public 项目，但已登录 User 或 Token 读取同一 public
  项目时，`can_access` 直接走 `project_permission`（必须是 owner 或
  collaborator），导致无授权关系时返回 403/404；同一个公开项目退出登录能访问、
  登录后反而不能访问。
- 根因：`server/src/permissions/checker.rs` 的 `Resource::Project` 分支仅
  `Principal::Anonymous` 检查 `visibility == Public`；User/Token 分支完全忽略
  public 可见性。
- 目标：与设计契约
  `docs/versions/v0.1/modules/filehub/001-filehub-core-platform/design/projects.md`
  的「list 按权限核心过滤可见项目：Anonymous 仅 public；User/Token 为 public +
  拥有项目角色/owner 的 private 项目」一致：已认证身份对 public 项目可读取，
  private 与写/管理动作仍要求授权关系。

## Scope

### In scope

- 修改 `server/src/permissions/checker.rs` 的 `Resource::Project` 分支：
  - `Principal::User`：public 项目放行只读动作
    （`metadata:read`/`artifacts:read`），与匿名一致；其余动作仍走
    `project_permission`；
  - `Principal::Token`：保持现有判定顺序 `project_scope -> token scope ->
    用户项目权限`，在 `project_scope` 与 token 读 scope 均通过后，public
    项目放行只读动作；不要求用户有 owner/collaborator 关系；
  - Anonymous、private 项目、写/管理动作行为不变。
- 新增/扩展回归测试：`server/tests/unit/permissions.rs`（User/Token 无授权
  读 public、Token 缺 scope 仍拒绝、写仍拒绝、Specified 范围外仍拒绝）与
  `server/tests/unit/projects.rs`（已登录 member 的 list 包含 public 项目）。
- 同一标准流程的 `docs/changes/029-public-read-logged-in.md` 与任务内
  `completion-report.md`。

### Out of scope

- 不改 Anonymous 与 private 项目的判定；
- 不改 `artifacts:write`/`administration` 等写与项目管理动作；
- 不改 token `project_scope` 的 fail-closed 语义（Specified 范围外的项目
  即使 public 仍拒绝，保持 025-token-project-scope-enforcement 已确认契约）；
- 不改 JWT/claims、数据库 schema、HTTP API 形状、CLI、前端或发布/CI 配置；
- 不回溯修订已批准的 001 设计文档（本任务在 change record 中记录本提案对
  `permissions.md`「User/Token 按账号级与项目角色继续判定」措辞的解释）。

### Boundary with neighboring modules

- 归属 001 权限核心（`permissions/checker.rs`）；`projects.list`、
  `versions.list/get` 经 `can_access(metadata:read)` 自动获得修复；下载侧
  `artifacts:read` 由同一判定覆盖。

## Requirement Review

- 需求合理：public 即可公开读取，「登录反而更受限」与设计意图相悖；现有
  permissions.md 措辞有歧义，本项目以用户引用的 projects.md:70 契约为准：
  User/Token 可见 public + 有权 private。写与管理动作仍严格按角色判定，
  不构成权限放大。
- 边界决策（Token）：保留 025 确立的 `project_scope` 硬限制与 token scope
  快照语义——public 只读不绕过 Specified 项目范围，也不绕过 token 缺少的
  读 scope；默认 `All` 范围 token（现状默认值）即可读所有 public。
- 注意：工作区已有 025 token 项目范围的未提交改动叠加在 checker.rs 上，本
  修复作为增量兼容实现。

## Proposal Items

| proposal_id | change_id | requirement | success_evidence |
|-------------|-----------|-------------|-----------------|
| P-001 | fh-permissions-public-read-login | `can_access` Project 分支：User/Token 在各自前置校验后对 public 放行 `metadata:read`/`artifacts:read`，private 与写/管理仍走 `project_permission` | 单元测试断言无授权 User、All 范围 token（带读 scope）读 public 放行；Specified 范围外 public、缺读 scope 与写/管理动作仍拒绝 |
| P-002 | fh-permissions-public-read-tests | 新增 User/Token 无授权读 public 回归断言并扩展项目列表过滤断言，覆盖 public/private、project_scope 与 token scope 边界 | `can_access_feature_and_project_matrix`、`token_project_scope_restricts_access_outside_scope`、`project_crud_and_visibility_flow` 定向运行通过 |

## Success Criteria

- 可见结果（`cargo test -p filehub-server --test unit` 定向运行通过）：
  - public 项目 `metadata:read`/`artifacts:read`：Anonymous、无授权 User、
    带读 scope 的 All 范围 Token 均为 true；
  - 无授权 User/Token 对 public 项目 `artifacts:write`/`administration`
    仍为 false；无授权身份对 private 项目读仍为 false；
  - Token：缺 `metadata:read`/`artifacts:read` scope 时 public 同动作仍
    拒绝；`Specified` 范围外 public 项目仍拒绝；
  - `projects.list`：member 登录后可见 public 项目。
- 证据：权限单元测试通过 + lower-tier-check completion 通过，完成报告记录
  缺陷搜索结论。
- 非目标：行为扩展至 private/写路径、契约/文档 API 变更、高风险管理流程。

## Risks

- Token public 读与 project_scope 的交互是最小争议点：本提案推荐
  `Specified` 范围外仍拒绝（维持 025 fail-closed）；若用户希望 public 完全
  绕过 project_scope，属于方案修改，需在确认时说明。
- permissions.md 措辞与 projects.md:70 的历史歧义已记录在本提案，不改已
  批准文档。
- 工作区存在在途未提交改动（025 相关），修复以增量补丁叠加，不改动那些
  文件的其他部分。
