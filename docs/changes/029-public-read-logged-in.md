# Public 项目登录后只读可见性修复（User/Token 分支补齐 public 可见性）

- Status: complete
- Owner module: filehub（server 权限核心）
- Task manifest: `docs/versions/v0.1/modules/filehub/029-public-read-logged-in/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/029-public-read-logged-in/proposal.md`
- Affected paths: `server/src/permissions/checker.rs`、`server/tests/unit/permissions.rs`、`server/tests/unit/projects.rs`
- Explicit tier override: none（用户确认 standard）
- Expanded high-risk packet: none

## Approach

- `SqlitePermissionChecker::can_access` 的 `Resource::Project` 分支增加 public
  只读放行：
  - `Principal::User`：`visibility == Public` 且动作为
    `metadata:read`/`artifacts:read` 时直接放行，不再要求 owner/collaborator；
  - `Principal::Token`：保持 `project_scope -> token scope` 前置硬校验
    （025 fail-closed 语义不变，`Specified` 范围外即使 public 仍拒绝；token
    缺读 scope 仍拒绝），通过后才放行 public 只读；
  - private 项目与写/管理动作仍走 `project_permission`，行为不变。
- 回归测试覆盖：无授权 User/Token 读 public、缺 scope 拒绝、Specified 范围
  内外 public 对照、写/管理仍拒绝、member 登录后项目列表包含 public。

## Risk Screen

- Public contract, protocol, or CLI change: no（API 请求/响应形状不变；行为回归
  到 design/projects.md「User/Token 可见 public + 有权 private」既定契约）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（仅把已公开内容的只读扩展到
  已认证身份；private 与写/管理边界不变；token project_scope 保持 fail-closed）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server`（24 个 unit + 2 个
  api_integration + 2 个 dv 全部通过），其中权限与项目用例含新增无授权
  public 读、Specified 范围内外 public、缺 scope、写/管理仍拒绝等断言；
  并跟踪 HTTP 读取/下载路由确认均经 `can_access(metadata:read)` 生效
- Result: pass
- Residual risk or follow-up: 无；token public 读与 project_scope 的交互边界已
  在提案确认时明确（Specified 范围外 public 仍拒绝）。
