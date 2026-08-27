---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-26
user_statement: 用户 2026-08-26 回复「修复吧」，确认 high-risk 全流程；测试按已确认提案与设计执行。
---

## Approval Record

- approver: user
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「修复吧」，确认采纳提案并按 high-risk
  全流程执行；测试阶段验证 decode_session 拒绝 refresh 类型且续期链路不变。

# refresh token 只能用于 refresh Testing

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| none | 测试集中在 `server/tests/unit/account.rs` 与 `server/tests/api_integration.rs`，不拆分独立 testing 子文档 | filehub-server 会话/refresh 类型边界 |

## Unified Test Entry

- Machine-readable task plan: `docs/versions/v0.1/modules/filehub/044-refresh-token-session-only/testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/044-refresh-token-session-only all`
- Single-task boundary: 只运行本任务 testplan 注册的 contract + unit + dv +
  integration 步骤，不运行模块级/`all all`/root 快捷入口/质量门禁。
- Registration: 新增断言在 `server/tests/unit/account.rs`
  （`decode_session_rejects_refresh_session`）与
  `server/tests/api_integration.rs`（`refresh_session_cannot_access_user_apis`），
  经 testplan.yaml 的 unit/integration 步骤统一可达；dv 步骤守护既有会话/生命周期
  回归。

## Red-Green Regression Evidence

- Red（修复前复现，均为临时移除 decode_session 拒绝分支后录制）：
  - `cargo test -p filehub-server --test unit_tests decode_session_rejects_refresh_session`
    失败：refresh token 被解码为 `FilehubAccount`，断言「must not decode as an
    access session」触发；
  - `cargo test -p filehub-server --test api_integration refresh_session_cannot_access_user_apis`
    失败：`GET /api/v1/projects` 携带 refresh token 返回 200（期望 401）。
- Green（恢复拒绝分支后）：
  - 同一单元/集成用例依次通过；任务级 `test-run.py filehub/044-refresh-token-session-only all`
    全绿（运行产物记录于 `.harness/test-results/test-runs/`）。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| account（vendored sfo-account 解码收口） | `decode_session` 拒绝 refresh 类型；`refresh_session` 仍只接受 refresh | none | decode(refresh) 返回 SessionInvalid；decode(session) 返回用户；refresh(refresh) 换发成功 | 普通 session 无 sub vs refresh 固定 sub；错误码类别 | unit | server/tests/unit/account.rs | covered | not-applicable |
| account（server 适配） | `AccountModule::decode_session` 薄适配透传拒绝语义 | none | refresh 拒绝、正常 session 放行、refresh 轮换可用 | 同上 | unit | server/tests/unit/account.rs | covered | not-applicable |
| http 认证桥 | `SessionAuthWrapper::decode_user` 不把 refresh 当用户身份 | none | refresh Bearer 访问 /api/v1 返回 401；session Bearer 照常 | 401 状态码、错误信封 | integration | server/tests/api_integration.rs | covered | not-applicable |
| sfo-account 用户信息路由 | `/account/get_account_info*` 复用 decode_session，不接受 refresh | none | 正常 session 200；refresh `err != 0` | 200 信封错误分类 | integration | server/tests/api_integration.rs | covered | not-applicable |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| session/refresh 类型判别 | decode_session 的 sub 分支（无 sub 放行 / refresh_sub 拒绝） | `cargo test --test unit_tests` | refresh 拒绝、session 放行 | unit | server/tests/unit/account.rs | covered | not-applicable |
| refresh 续期闭环 | refresh_session 换发 + 新 session 可用 | `cargo test --test unit_tests`、`api_integration` | 换发 err=0；新 session 200 | unit/integration | account.rs、api_integration.rs | covered | not-applicable |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| `/api/v1/projects` | 认证桥凭据分类 | session / 换发后 session 访问 200 | refresh token Bearer 401 | integration | server/tests/api_integration.rs | covered | not-applicable |
| `/account/get_account_info` | sfo-account 用户信息 | session Bearer 200 | refresh token `err != 0` | integration | server/tests/api_integration.rs | covered | not-applicable |
| `/account/refresh_session` | 续期端点 | refresh token 换发 err=0 | 普通 session 不可换发（既有行为，sfo-account 收口） | unit/integration | account.rs、api_integration.rs、sfo-account 既有测试 | covered | not-applicable |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-refresh-decoder-reject | `design/account-refresh.md` File-Level Interfaces（decode_session 拒绝 refresh_sub 分支）与消费方自动生效 | VAL-refresh-reject | unit | refresh-session-unit | no | |
| fh-refresh-regression | `testing.md`/testplan：回归断言、red-green 复现、API 反例与续期闭环 | VAL-refresh-regression | unit | refresh-session-unit | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-refresh-decoder-reject | normal | yes | VAL-refresh-reject | unit | covered | not-applicable |
| fh-refresh-decoder-reject | boundary | yes | VAL-refresh-reject | unit | covered | 无 sub 的普通 session 与固定 refresh_sub 的分界正是判别条件 |
| fh-refresh-decoder-reject | negative | yes | VAL-refresh-reject-api | integration | covered | not-applicable |
| fh-refresh-decoder-reject | error | yes | VAL-refresh-reject | unit | covered | 断言返回 SessionInvalid 错误码 |
| fh-refresh-decoder-reject | compatibility | no | VAL-refresh-reject-api | integration | not-applicable | 公开契约/签名不变，无兼容回归面；收紧的是缺陷能力 |
| fh-refresh-decoder-reject | lifecycle | yes | VAL-refresh-reject | unit | covered | refresh 轮换仍成功（unit），新 session 可用（integration） |
| fh-refresh-decoder-reject | cross-module | yes | VAL-refresh-reject-api | integration | covered | HTTP + sfo-account + permissions 跨模块认证链 |
| fh-refresh-regression | normal | yes | VAL-refresh-regression | unit | covered | not-applicable |
| fh-refresh-regression | boundary | yes | VAL-refresh-reject | unit | covered | 类型分界（无 sub / refresh_sub） |
| fh-refresh-regression | negative | yes | VAL-refresh-reject-api | integration | covered | API 反例（401 / err != 0） |
| fh-refresh-regression | error | yes | VAL-refresh-reject | unit | covered | SessionInvalid 断言 |
| fh-refresh-regression | compatibility | no | VAL-refresh-regression-api | integration | not-applicable | 无公开契约变化；续期行为保持既有断言 |
| fh-refresh-regression | lifecycle | yes | VAL-refresh-regression-api | integration | covered | 登录 -> refresh 换发 -> 新 session 访问 |
| fh-refresh-regression | cross-module | yes | VAL-refresh-reject-api | integration | covered | 同一反例覆盖认证桥与 sfo-account 两条消费路径 |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | `design/account-refresh.md` File-Level Interfaces（token.sub 判别域） | 无 sub 普通 session 放行、`sub == refresh_sub` 拒绝 | unit | covered | not-applicable |
| parameter-domain | 自定义 `session_sub` / 非法配置组合 | sfo-account 既有配置校验用例 | unit | gap | vendored crate 非 workspace 成员，无法在本仓库用 canonical 入口执行其 inline 测试而不改其 manifest（超出本任务范围）；`SessionConfig::validate` 已静态封死同 sub 组合，风险可接受 |
| state-transition | `design.md` State and Ownership（refresh -> authenticated 拒绝 / refreshed 允许） | decode(refresh) 拒绝；refresh(refresh) 换发后新 session 可用 | unit | covered | not-applicable |
| invariant | `design.md` Invariants（普通 session 解码/过期语义不变） | 普通 session decode 成功 | unit | covered | not-applicable |
| error-handling | `design.md`（SessionInvalid 错误类别复用） | API 401 / `err != 0` 映射 | integration | covered | not-applicable |
| failure-path | `design/account-refresh.md`（decode 验签/过期既有失败路径） | 既有 garbage bearer 失败断言继续通过 | unit | covered | not-applicable |
| concurrency | `design.md`（无新增共享可变状态） | not-applicable: 新增分支是纯只读 claims 比较，无并发/重入/顺序声明 | unit | not-applicable | 设计未引入并发边界，无对应派生用例 |

## Validation Rationale

| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| refresh token 不能映射为用户身份 | 单测断言 decode(refresh) 为 SessionInvalid；集成断言用户接口 401 / `err != 0` | 在解码收口与两条 HTTP 消费路径同时断言，防实现回退与消费方分叉 | not-applicable |
| 正常 session 不受影响 | 单测 decode(session) 返回 alice；集成 session 访问 200 | 直接覆盖判别分支的另一侧，防误伤 | not-applicable |
| refresh 续期闭环 | 单测 refresh(refresh) 换发；集成 refresh 端点 err=0 且新 session 200 | 覆盖状态迁移的允许侧 | not-applicable |
| 既有行为回归 | 全套 unit/dv/integration 通过 | 登录、token 生命周期、项目/版本/上传等既有断言不变 | not-applicable |

## Unit Tests

| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| `DefaultAccountManager::decode_session` | `token.sub == Some(refresh_sub)` | 返回 SessionInvalid，不返回用户 | server/tests/unit/account.rs | covered | not-applicable |
| `DefaultAccountManager::decode_session` | `token.sub` 为 None（普通 session） | 仍解码用户 | server/tests/unit/account.rs | covered | not-applicable |
| `DefaultAccountManager::refresh_session` | refresh token 输入 | 换发新 session/refresh | server/tests/unit/account.rs | covered | not-applicable |
| `AccountModule::decode_session` | 薄适配透传 | refresh 拒绝、session 放行 | server/tests/unit/account.rs | covered | not-applicable |

## DV Tests

| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| 登录/会话/项目主工作流 | main | dv_full_workflow_with_tokens_and_gc | 主流程全绿（登录 session 可用） | server/tests/dv_tests.rs | covered | not-applicable |
| 登录/session/token 生命周期 | lifecycle | dv_full_workflow | 既有全绿 | server/tests/dv_tests.rs | covered | 本修复不引入新状态机，dv 作为回归守护 |
| 越权/无效凭据失败路径 | failure | dv 既有失败断言 | 拒绝语义保持 | server/tests/dv_tests.rs | covered | not-applicable |

## Integration Tests

| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| `/api/v1/projects` + Bearer | http（认证桥）+ account + permissions | 普通/换发后 session 200 | refresh token 401 | server/tests/api_integration.rs | covered | not-applicable |
| `/account/get_account_info` + Bearer | sfo-account + account | session 200 | refresh token 信封 `err != 0` | server/tests/api_integration.rs | covered | not-applicable |
| `/account/refresh_session` + Bearer | sfo-account | refresh token 换发 err=0；新 session 可访问 | 普通 session 不可换发（既有 sfo-account 语义） | server/tests/api_integration.rs + sfo-account 既有测试 | covered | not-applicable |

## Definition of Done

- [x] Testing docs 覆盖全部两个 change_id 与受影响子模块（account、sfo-account、http 认证桥、账户信息路由）
- [x] `testplan.yaml` 与 `testing.md` 引用一致，unit/dv/integration + contract 均注册
- [x] 新测试位于专用测试文件（`server/tests/unit/account.rs`、`server/tests/api_integration.rs`），经任务 testplan 统一可达
- [x] Bugfix 提供 red-green 回归证据（移除/恢复拒绝分支后同用例红转绿）
- [x] 每个实现 change_id 均有直接验证与用例类型覆盖
- [x] 任务级 `test-run.py filehub/044-refresh-token-session-only all` 全绿
