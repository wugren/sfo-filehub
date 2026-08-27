---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-23
---

# Token 权限服务端化 Testing

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| none | 本任务测试集中在 `server/tests/unit/` 与既有 dv/integration 回归，不拆分独立 testing 子文档 | 文件集散服务端 token 授权链路 |

## Unified Test Entry

- Machine-readable task plan: `docs/versions/v0.1/modules/filehub/025-token-project-scope-enforcement/testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/025-token-project-scope-enforcement all`
- Single-task boundary: 只运行本任务 testplan 注册的三个步骤，不运行模块级/`all all`/root 快捷入口/质量门禁。
- Registration: 新增断言全部在 `server/tests/unit/`（`unit_tests` 目标）中，
  经 testplan.yaml 的 unit/dv/integration 三个步骤统一可达。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| tokens | JWT 载荷收窄 + resolve 数据库权威 + 重签/轮换 | none | create 的 JWT data 载荷不含 scopes/project_scope；resolve 返回的 scopes/project_scope 与 DB 一致；scope/项目范围变更重签后旧 JWT 失效 | 指定项目集合、All 缺省、claims 缺失权限字段、update/rotate 后权限跟随 DB | unit | server/tests/unit/tokens.rs | covered | not-applicable |
| permissions | checker 项目范围 fail-closed 校验 | none | Specified 集合外项目全部动作拒绝；集合内项目按既有 scope+用户权限放行；All 行为不变 | 范围外 read/write/admin、All 对照 | unit | server/tests/unit/permissions.rs | covered | not-applicable |
| http | TokenAuthWrapper 透传 project_scope | none | resolve 结果原样映射进 Principal::Token | 失败返回 None（既有路径） | unit/integration | server/tests/api_integration.rs | covered | not-applicable |
| model | Principal::Token 变体新增 project_scope | none | 构造点全仓库同步 | 无缺省值（防止隐式 All） | unit | server/tests/unit/versions.rs、permissions.rs | covered | not-applicable |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| token 权限数据库权威 | create/update/rotate/resolve 全路径 | `cargo test --test unit_tests` | JWT 无权限字段、resolve 与 DB 一致、旧 JWT 失效 | unit | server/tests/unit/tokens.rs | covered | not-applicable |
| project_scope 授权矩阵 | Specified/All x 项目 x 动作 | `cargo test --test unit_tests` | 范围外拒绝、范围内放行、All 不变 | unit | server/tests/unit/permissions.rs | covered | not-applicable |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| v1 HTTP token/项目/版本接口 | 对外 JSON 契约不变 | 既有 token 创建/列表/轮换、项目/版本访问流程全绿 | 未经授权 401/403（既有断言） | integration | server/tests/api_integration.rs | covered | not-applicable |
| token JWT claims | 服务端内部凭据格式收窄 | 新 JWT data 不含 scopes/project_scope | 旧 claims 未知字段自然忽略（serde，未加兼容层） | unit | server/tests/unit/tokens.rs | covered | not-applicable |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-token-permissions-server-side | `design/tokens.md` + `design/model.md` + `design/http.md`；实现 server/src/tokens/{model,service}.rs、model/principal.rs、http/auth.rs | VAL-token-db-authority | unit | token-permissions-unit | no | |
| fh-token-project-scope-enforce | `design/permissions.md`；实现 server/src/permissions/checker.rs | VAL-token-project-scope | unit | token-permissions-unit | no | |
| fh-token-project-scope-tests | 测试与契约：server/tests/unit/*、docs/api/v1-contract.md | VAL-token-permission-regression | unit | token-permissions-unit | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-token-permissions-server-side | normal | yes | VAL-token-db-authority | unit | covered | not-applicable |
| fh-token-permissions-server-side | boundary | yes | VAL-token-db-authority | unit | covered | not-applicable |
| fh-token-permissions-server-side | negative | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-permissions-server-side | error | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-permissions-server-side | compatibility | no | VAL-token-db-authority | unit | not-applicable | 用户明确要求不考虑兼容；不做旧 JWT 特判，仅断言新 claims 不含权限字段 |
| fh-token-permissions-server-side | lifecycle | yes | VAL-token-db-authority | unit | covered | not-applicable |
| fh-token-permissions-server-side | cross-module | yes | VAL-token-permission-regression | integration | covered | not-applicable |
| fh-token-project-scope-enforce | normal | yes | VAL-token-project-scope | unit | covered | not-applicable |
| fh-token-project-scope-enforce | boundary | yes | VAL-token-project-scope | unit | covered | not-applicable |
| fh-token-project-scope-enforce | negative | yes | VAL-token-project-scope | unit | covered | not-applicable |
| fh-token-project-scope-enforce | error | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-project-scope-enforce | compatibility | no | VAL-token-project-scope | unit | not-applicable | 授权判定行为收紧属预期语义变化，不做兼容回退 |
| fh-token-project-scope-enforce | lifecycle | yes | VAL-token-project-scope | unit | covered | not-applicable |
| fh-token-project-scope-enforce | cross-module | yes | VAL-token-permission-regression | integration | covered | not-applicable |
| fh-token-project-scope-tests | normal | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-project-scope-tests | boundary | yes | VAL-token-db-authority | unit | covered | not-applicable |
| fh-token-project-scope-tests | negative | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-project-scope-tests | error | yes | VAL-token-permission-regression | unit | covered | not-applicable |
| fh-token-project-scope-tests | compatibility | no | VAL-token-db-authority | unit | not-applicable | 用户明确要求不考虑兼容，无兼容断言 |
| fh-token-project-scope-tests | lifecycle | yes | VAL-token-db-authority | unit | covered | not-applicable |
| fh-token-project-scope-tests | cross-module | yes | VAL-token-permission-regression | integration | covered | not-applicable |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | `design/tokens.md` File-Level Interfaces（TokenPayload/TokenPrincipal/ProjectScope） | VAL-token-db-authority（Specified/All、claims 无权限字段） | unit | covered | not-applicable |
| state-transition | `design/tokens.md` State and Ownership（create/update/rotate） | VAL-token-db-authority（重签后 DB 权限生效、旧 JWT 失效） | unit | covered | not-applicable |
| invariant | `design.md` Overall Approach（数据库唯一授权权威 + fail-closed 判定顺序） | VAL-token-project-scope（范围外全拒、All 不变） | unit | covered | not-applicable |
| error-handling | `design.md` Key Flows（resolve 失败 -> 认证失败） | VAL-token-permission-regression（撤销/轮换/越权既有断言） | unit | covered | not-applicable |
| failure-path | `design/tokens.md` resolve 失败路径（无 token/已撤销/验签失败） | VAL-token-permission-regression（revoke/rotate 后旧 JWT 失效既有断言） | unit | covered | not-applicable |
| concurrency | `design/tokens.md` rotate/revoke 密钥替换顺序 | VAL-token-db-authority（update 重签后旧 JWT 立即失效） | unit | covered | not-applicable |

## Validation Rationale

| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| JWT 不再携带权限 | base64 解码签发 JWT 的 `data` 声明 | 直接在凭据端断言无 scopes/project_scope 字段，防双源回潮 | not-applicable |
| resolve 数据库权威 | create/update 后 resolve 与 DB 值逐项断言 | 覆盖签发与重签两条写入路径 + 一条读取路径 | not-applicable |
| project_scope fail-closed | 双项目矩阵：集合内/外 x read/write/admin + All 对照 | 能放行/拒绝两个方向的错误都会被抓到 | not-applicable |
| 无回归 | 既有 unit/dv/integration 全套 | token 生命周期、版本发布、HTTP 契约断言全部保持 | not-applicable |

## Unit Tests

| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| TokenService::create + JWT 载荷 | scopes/project_scope 提交 | claims data 不含权限字段 | tests/unit/tokens.rs | covered | not-applicable |
| TokenService::resolve | 数据库读取路径 | scopes/project_scope 与 DB 一致 | tests/unit/tokens.rs | covered | not-applicable |
| TokenService::update 重签 | scope/project_scope 变更 | 旧 JWT 失效，resolve 跟随 DB | tests/unit/tokens.rs | covered | not-applicable |
| PermissionChecker::can_access | Project x Token x Specified/All | 范围外全动作拒绝、范围内放行、All 不变 | tests/unit/permissions.rs | covered | not-applicable |
| Principal::Token 构造点 | 全仓库现有构造 | 新增 project_scope 字段无遗漏 | tests/unit/permissions.rs、versions.rs | covered | not-applicable |

## DV Tests

| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| token 生命周期（create/update/rotate/revoke） | lifecycle | dv_full_workflow_with_tokens_and_gc | 生命周期与密钥替换保持全绿 | tests/dv_tests.rs | covered | not-applicable |
| 主工作流（token 生命周期与二次限制） | main | dv_full_workflow_with_tokens_and_gc | 创建/读取/撤销流程与权限限制保持全绿 | tests/dv_tests.rs | covered | not-applicable |
| 越权/限制失败路径 | failure | dv_full_workflow 内失败断言 | 越权拒绝、无效凭据失败 | tests/dv_tests.rs | covered | not-applicable |
| 持久化恢复 | persistence | dv_persistence_across_reopen | 权限字段仍由 DB 承载 | tests/dv_tests.rs | covered | not-applicable |

## Integration Tests

| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| token 生命周期与二次限制 HTTP | http + tokens + permissions | 创建 201、read token 可读 | 旧 token 401、写 403 | tests/api_integration.rs | covered | not-applicable |
| 项目/版本/下载访问边界 | http + projects + versions + permissions | 授权访问全绿 | 越权/未授权状态码正确 | tests/api_integration.rs | covered | not-applicable |

## Definition of Done
- [x] Testing docs 覆盖全部三个 change_id 与受影响子模块（tokens/permissions/http/model）
- [x] `testplan.yaml` 与 `testing.md` 引用一致，三层均注册单步
- [x] 新测试注册在任务 testplan，经 `test-run.py filehub/025-token-project-scope-enforcement all` 可达
- [x] 单元测试位于 `server/tests/unit/`，DV 在 `tests/dv_tests.rs`，集成在 `tests/api_integration.rs`
- [x] 本任务只运行任务级 testplan，不选择模块级/`all all`/质量门禁
- [x] 每个实现 change_id 均有直接验证与用例类型覆盖
- [x] `cargo test -p filehub-server` 全部通过（21 unit + 2 dv + 2 integration）
