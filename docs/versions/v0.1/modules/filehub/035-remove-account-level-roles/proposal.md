task_manifest: task.yaml
status: approved
---

# 移除账号级 owner/member 角色，项目归属由创建者持有

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 回复「确认」，确认采纳提案（含 owner-only 删除边界）并接受建议的 standard 层级。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本任务是生产行为变更，且处于授权边界内（`projects:create`/`projects:delete`
    的判定矩阵、`Principal::User` 结构、`account_roles` 表与配置 `[users].role`），
    不满足 trivial 对"无 security/privacy 实质影响、无 schema/配置变更"的界定，
    按 standard 默认 bounded refactor 流程执行（pre-edit 基线 -> 实现 -> 验证 ->
    变更记录 -> completion-report）。
  - 未发现 high-risk 触发边界：不改公开 API/协议/CLI 契约与前端契约（admin-web
    只使用项目角色 `read/write/admin`，不暴露账号角色）；不新增依赖；不改登录/会话
    校验与 token 签名机制；权限判定整体向"项目 owner 唯一持有删除权"收敛、
    删除权限反而比现状收紧（现状账号级 owner 可删任意项目）。`account_roles` 表
    与 `[users].role` 属于本仓库 greenfield 数据模型，随本任务从 schema 与配置中
    移除，并同步更新仓库内唯一配置文件。

## Background and Goal

- 现状冲突：代码与设计文档中同时存在两套"角色"概念——账号级
  `AccountRole { Owner, Member }`（存 `account_roles`，由配置 `[users].role`
  初始化，仅用于 `projects:create`/`projects:delete` 两个 feature 动作）与项目级
  关系（`projects.owner` 隐式 admin，`project_grants` 上的
  `read/write/admin` 协作者）。用户明确：**owner/member 是针对项目的**，单纯账号
  不存在 owner/member 区分，任何账号都能创建自己的项目。
- 目标：
  - 删除账号级 `AccountRole` 及其全部数据/配置/判定路径（`account_roles` 表、
    `[users].role`、`Principal::User.account_role`、`role_for_user`、auth 包装器
    的角色补齐）。
  - `projects:create` 对所有已登录账号开放（Anonymous 与无 `projects:create`
    scope 的 token 仍拒绝）；创建者即为项目 owner。
  - `projects:delete` 改为项目级判定：仅目标项目 owner 可删除；admin 协作者不可
    删除（沿用既有测试结论，原因从"账号级 owner 限制"改为"项目级 owner 限制"）；
    token 删除仍需显式 scope（见 Proposal Items）。

## Scope

### In scope

- `server/src/model/role.rs`：删除 `AccountRole` 枚举及 Display/FromStr/parse；
  `model/mod.rs` 导出同步；
- `server/src/model/principal.rs`：`Principal::User` 去掉 `account_role` 字段，
  变体简化为 `User { user_id }`；
- `server/src/model/config.rs`：`UserConfig` 删除 `role` 字段；`filehub-server.json`
  删除 `"role": "owner"`；
- `server/migrations/0003_roles_grants.sql`：删除 `account_roles` 建表，保留
  `project_grants`（安全范围内保留 SQLite 中已存在但不被引用的旧表不动即可）；
- `server/src/permissions/mod.rs`：`PermissionsModule::init` 移除按配置 upsert
  账号角色的循环与 `role_for_user`；
- `server/src/permissions/checker.rs`：删除 `role_for_user`/`user_account_role`；
  Feature 分支改为 User 直接允许 `projects:create`、Token 仅校验 scope；
  Project 分支新增 `projects:delete` 动作——owner 允许、非 owner（含 admin
  协作者）拒绝，token 还需携带 `projects:delete`（及沿用现状的
  `administration` 项目级校验）且所属用户为目标项目 owner、范围覆盖目标项目；
- `server/src/http/auth.rs`、`server/src/contract/mod.rs`、
  `server/src/account/authn.rs`：删除 `role_of` 桥与占位角色构造，认证包装直接
  产出 `Principal::User { user_id }`；
- `server/src/projects/service.rs`：`create` 保持 `projects:create` feature 判定
  （语义放宽到所有已登录账号）；`delete` 改为项目级判定（见上）；
- 测试同步：`server/tests/unit/permissions.rs`（角色初始化用例删除/改写为
  "任意账号可创建"）、`projects.rs`（member 可创建并成为 owner；admin 协作者
  不可删除；token 删除矩阵按新 owner 语义调整）、`model.rs`（AccountRole 解析
  用例删除）、`account.rs` 与 `tests/common/mod.rs`（配置去掉 role）、
  `api_integration.rs`（member 创建由 403 改为 201 且成为 owner）、
  `dv_tests.rs`/`versions.rs`（Principal 构造去掉 account_role）；
- 文档同步：`docs/versions/v0.1/modules/filehub/001-filehub-core-platform/design/`
  （permissions.md 矩阵与 Design Notes、design.md/model.md/testing.md 中的
  账号角色描述）与 `docs/modules/filehub.md` 输入说明；新增
  `docs/changes/035-remove-account-level-roles.md` 与任务包
  `completion-report.md`。

### Out of scope

- 不改变登录/会话/token 签名与过期机制；
- 不改变项目级协作者角色（`read/write/admin`）的授权矩阵（除删除权限归属）；
- 不引入账号注册/邀请流程；账号仍来自配置初始化；
- 不重写历史已批准提案（001-proposal.md 等）作为历史记录保留；
- 不处理已运行过的部署库中遗留的空 `account_roles` 表（不引用即失效，无需
  DROP 迁移；如需彻底删除旧表可另立任务）。

## Requirement Review

- 需求合理性：合理且更符合"项目属于创建者"的产品语义，同时消除一个仅服务于
  创建/删除门槛、且让"普通账号不能建项目"的隐式配置角色。
- 方向选择：
  - 删除而非保留兼容层：用户不要求兼容旧配置，仓库内唯一配置随本任务更新；
    删除 `[users].role` 后，携带该键的旧配置会在启动时被 serde 拒绝，属预期内
    的配置迁移，已列入 Risks。
  - 删除权限归属：采用 owner-only（推荐），保持"创建者所有"语义并收紧现状
    （现状账号级 owner 可删任意项目）。admin 协作者沿用既有结论不可删除。
  - Token 语义：创建/删除 token 能力仍由显式 scope 表达，仅去掉"所属用户必须是
    账号级 owner"这一层，删除改为"所属用户必须是项目 owner"。
- 剩余待确认问题：删除项目的授权边界是否采用 owner-only（本提案推荐），还是
  允许 admin 协作者也可删除。若用户只回复"确认"，按 owner-only 执行。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认」，确认采纳含
  owner-only 删除边界的提案并以 standard 终值批准；随后按 lower-tier 流程执行
  （pre-edit 基线 -> 实现 -> 验证 -> 变更记录 -> completion-report）。

## Proposal Items

| proposal_id | change_id | requirement | success_evidence |
|-------------|-----------|-------------|-----------------|
| P-001 | fh-remove-account-level-roles | 删除 `AccountRole` 类型、`account_roles` 表、`[users].role` 配置及其注入/判定路径；`Principal::User` 不再携带账号角色 | 源码与配置不再出现账号角色引用；`cargo test` 通过；`account_roles` 建表从 0003 移除 |
| P-002 | fh-project-create-open-to-all | `projects:create` 对任意已登录账号放行，创建者成为项目 owner；Anonymous 拒绝；token 仍需 `projects:create` scope，不再叠加账号级 owner 判定 | 单元/集成断言任意账号创建 201 且 owner 为创建者；无 scope token/Anonymous 仍拒绝 |
| P-003 | fh-project-delete-owner-only | `projects:delete` 改为项目级动作：仅项目 owner 可删（admin 协作者不可删）；token 需 `projects:delete` scope（并沿用 `administration` 项目级校验）且所属用户为目标项目 owner、token 范围覆盖目标项目 | projects.rs 删除矩阵用例（非 owner/缺 scope/范围外/非 owner token 拒绝；owner 成功）通过 |
| P-004 | fh-role-cleanup-tests-docs | 同步更新单元/集成/DV/前端契约相关测试与权限设计/模块文档，删除账号角色描述并锁新矩阵 | `cargo test -p filehub-server` 与 admin-web 单测全绿；实时 docs 中不再出现账号角色描述 |

## Success Criteria

- `cargo test -p filehub-server` 全量通过；`admin-web` 既有测试通过（不改前端
  逻辑时无需新增前端用例）；
- 单元/集成中"member 不能创建项目 403"全部被"任意账号可创建并成为 owner"
  替代；`role_initialization_owner_and_member` 用例删除或改写；
- 删除权限测试覆盖：非 owner 用户/仅 `administration` scope 的 token 不可删除；
  项目 owner 可删除自己项目；token（带 `projects:delete`+`administration`、所属
  用户为 owner、范围覆盖）可删除；范围外与缺 scope 一律拒绝；
- 配置文件 `filehub-server.json` 与测试配置不再含 `role`；源码中不再出现
  `AccountRole`/`account_role`/`account_roles` 引用（注释与历史文档除外）；
- 按 standard 流程产出 `docs/changes/035-remove-account-level-roles.md` 与
  `completion-report.md`。

## Risks

- 授权边界：`projects:create` 对全员开放是用户明确要求；`projects:delete`
  同时收紧为 owner-only，避免"账号级 owner 可删任意项目"的现状放量面。若用户
  选择"admin 协作者也可删"，范围仅 P-003 变化。
- 配置兼容：删除 `[users].role` 后，带该键的旧配置文件启动失败并给出 serde
  错误；仓库内配置同步更新，文档记录迁移说明。
- 存量数据：已运行过 `0003` 的库会残留空 `account_roles` 表，本任务不引用该表，
  不影响运行；不做 DROP 迁移。
- 回归面：`Principal` 结构变化会触及 http/contract/所有测试构造，已列入 in-scope
  并通过全量 `cargo test` 验证；admin-web 不消费账号角色，前端契约无变化。
