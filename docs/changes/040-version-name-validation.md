# 版本名最小安全拒绝集校验：保留字 latest 与路由/响应头不安全字符

- Status: complete
- Owner module: filehub（filehub-server）
- Task manifest: docs/versions/v0.1/modules/filehub/040-version-name-validation/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/040-version-name-validation/proposal.md
- Affected paths: server/src/versions/service.rs；server/tests/unit/versions.rs；docs/api/v1-contract.md
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `SqliteVersionService::validate_version` 实现最小拒绝集：trim 后的版本名为空
  或字面值 `latest` 时拒绝；原始输入（trim 前）含 `/`、`?`、`#` 或任意 Unicode
  控制字符（`char::is_control()`，覆盖 C0/DEL/C1）时拒绝。字符扫描放在 trim
  之前，避免 `latest`/存储前的边缘控制字符被 `trim()` 剥除而绕过校验。
- `create_version` 在事务与权限检查前完成校验并返回 422 `invalid_input`；
  校验通过后沿用既有 trim + `BEGIN IMMEDIATE` 事务写入，不改变存储语义。
- 按用户确认口径不限制版本格式与长度（`1.0`、`Latest`、含空格等名称放行），
  读取/下载路由的 `latest` 关键字语义不变。
- 契约文档同步说明拒绝集与 422；单元回归覆盖拒绝集、放行集与“trim 后命中
  保留字/空值”边界。

## Risk Screen

- Public contract, protocol, or CLI change: yes —— `POST /versions` 输入集收紧
  （拒绝 `latest`、`/`、`?`、`#` 与控制字符，非法 422；422 错误码已有，非新增
  端点/错误码）。该变更属已确认提案范围。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no —— 关闭既有输入校验缺口，不
  新增信任面；不改变 `latest` 读取语义。
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests` 48 项全通过
  （含新增 `create_version_rejects_reserved_and_route_unsafe_names`）；
  `dv_tests` 2 项通过；`api_integration::upload_security_boundaries` 单独运行
  通过。
- Result: pass
- Residual risk or follow-up: 全量并行 `api_integration` 时
  `upload_security_boundaries` 与另一在制任务新增的 `project_list_pagination`
  集成用例共享测试服务器状态产生 409 干扰（单独运行通过），与本任务改动无关；
  该并行干扰收敛后可在全量模式下复跑确认。
