# Token 删除项目补项目级校验与空项目集合语义

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/027-token-delete-project-scope/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/027-token-delete-project-scope/proposal.md
- Affected paths: server/src/projects/service.rs, server/src/model/scope.rs, server/src/tokens/service.rs, server/tests/unit/projects.rs, server/tests/unit/tokens.rs, docs/api/v1-contract.md
- Explicit tier override: 用户 2026-08-23 回复「确认，按standard任务完成就好」，明确选择 standard（提案建议 high-risk）
- Expanded high-risk packet: none

## Approach

- `projects.service.rs::delete` 对 `Principal::Token` 在既有 Feature 级
  `projects:delete` 校验后追加项目级校验：复用
  `checker.can_access(Resource::Project(id), ACTION_ADMIN)`，即要求 token
  `project_scope` 包含目标项目、token 具 `administration` scope、所属用户为
  项目 owner/admin；项目不存在返回 404、无权限返回 403（与 set_visibility
  语义一致）。User session 删除路径不变。
- 空项目集合语义：`ProjectScope::Specified(vec![])` 等价于 `All`。
  `model/scope.rs` 新增 `normalize()`（空集合 -> All）并让 `FromStr` 对空值
  返回 `All`；`tokens/service.rs` create/update 落库前归一化，DB 存 `"all"`，
  兼容旧空串数据。All 语义仍受 checker 的用户项目权限限制，不放大越权。
- JWT/权限链路沿用 025 的数据库权威设计，本任务不重新引入 claims 权限属性。

## Risk Screen

- Public contract, protocol, or CLI change: no（HTTP JSON 请求/响应形状不变；
  空集合语义作为行为契约补充说明写入 docs/api/v1-contract.md）
- Persistent data, schema, or migration change: no（无表结构/迁移；既有空串值
  按 All 解析）
- Security, privacy, or trust-boundary change: yes——token 删除从仅 Feature
  级收紧为项目级授权校验（授权边界变更）；用户明确选择 standard 层级，
  剩余风险见 Residual risk or follow-up
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no（存量为「仅 projects:delete scope 的 token」的删除行为变化属预期收紧，非发布/回滚面）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests`（新增
  token_delete_requires_project_scope_and_project_admin 与
  empty_project_scope_means_all，23/23 通过）；`cargo test -p filehub-server`
  全套（23 unit + 2 dv + 2 integration）通过
- Result: pass
- Residual risk or follow-up:
  - 存量 token 若只有 `projects:delete` scope、缺少 `administration` scope，
    删除项目将返回 403（预期收紧；管理端表单可勾选 administration）；
  - 「空集合 = All」的放宽不放大越权，All 仍以 token 所属用户项目权限为界；
  - User session 删除语义未改，未引入后续项。
