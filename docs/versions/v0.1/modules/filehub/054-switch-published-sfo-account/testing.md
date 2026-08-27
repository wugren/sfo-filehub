---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-27
user_statement: 用户 2026-08-27 确认 high-risk 全流程并选择方案 2；测试按已确认
  提案与设计验证 crates.io sfo-account 0.2.1 语义与配置校验。
---

## Approval Record

- approver: user
- approval_date: 2026-08-27
- user_statement: 用户 2026-08-27 确认 high-risk 全流程并选择方案 2；测试按
  已确认提案与设计验证 crates.io sfo-account 0.2.1 语义与配置校验。

# sfo-account 0.2.1 依赖切换 Testing

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| none | 测试集中在 `server/tests/unit/config.rs`、`server/tests/unit/account.rs`、`server/tests/dv_tests.rs`、`server/tests/api_integration.rs`、`cli/tests/e2e_cli_server.rs` | 依赖来源、启动校验、登录失败/限流语义、cli e2e |

## Unified Test Entry

- Machine-readable task plan: `testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python
  ./harness/scripts/test-run.py filehub/054-switch-published-sfo-account all`
- Single-task boundary: 只运行本任务 testplan 的 contract + unit + dv +
  integration 步骤，不运行模块级/`all all`/root 快捷入口。
- Registration: 新增/修改断言位于专用测试文件
  `server/tests/unit/config.rs`（32 字节边界）与既有
  `server/tests/unit/account.rs`、`server/tests/api_integration.rs`、
  `cli/tests/e2e_cli_server.rs`（fixture 密钥与语义收敛），均经 testplan.yaml
  统一可达。

## Red-Green Regression Evidence

- 本任务是依赖来源与语义切换，不是缺陷修复；「Red」基线是切换到 0.2.1 后、
  断言适配前的已知红（proposal 证据表）：未知账号 err=9 vs 本地 shim err=10、
  限流英文消息、30 字节测试密钥启动 panic。
- Green：断言/fixture 适配后，任务级 `test-run.py
  filehub/054-switch-published-sfo-account all` 全部步骤成功
  （产出 `.harness/test-results/test-runs/<ts>-filehub+054-...-all.json`）。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| model（config） | `UsersConfig::validate` 最短 32 字节 | none | >=32 通过；<32 拒绝且不回显密钥 | 30/31 字节拒绝、32 字节边界 | unit | server/tests/unit/config.rs | covered | not-applicable |
| account（装配） | `AccountModule::init` 非 panic 组装 | none | 32 字节 key 正常装配；短 key 错误传播 | 构造失败路径 | unit | server/tests/unit/account.rs + dv_tests.rs | covered | not-applicable |
| account（登录语义） | 未知/密码错误/限流 0.2.1 语义 | none | err=9/10/11 与英文消息 | 未知 vs 密码错误区分 | unit/integration | unit/account.rs、api_integration.rs | covered | not-applicable |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| session_key 校验 | `<32` / `>=32` 分支 | `cargo test --test unit_tests` | 拒短放长、错误不回显密钥 | unit | server/tests/unit/config.rs | covered | not-applicable |
| 登录失败语义 | err=9/10 与消息 | `cargo test --test unit_tests`、`api_integration` | 按 0.2.1 区分 | unit/integration | unit/account.rs、api_integration.rs | covered | not-applicable |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| `POST /account/login` | 登录失败语义 | 登录成功 err=0 | 未知账号 err=9、密码错误 err=10、限流 err=11（英文消息） | integration | server/tests/api_integration.rs | covered | not-applicable |
| `/api/v1/*` + Bearer | 会话凭据 | session 200 | refresh token 401 / err!=0 | integration | server/tests/api_integration.rs | covered | not-applicable |
| cli e2e 服务器会话 | 32 字节 fixture | e2e 登录/发布/下载全流程 | 无（fixture 密钥边界由单元覆盖） | integration | cli/tests/e2e_cli_server.rs | covered | not-applicable |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-sfo-account-published-source | design.md Overall Approach / Directly Mapped Change Items | VAL-src | unit | sfo-account-unit | no | |
| fh-sfo-account-conformance | design/account-dependency.md File-Level Interfaces + design.md Key Flows / v1 契约 | VAL-config-unit（登录/限流集成见 Case-Type Coverage VAL-login-integration） | unit | sfo-account-unit | no | |
| fh-sfo-account-regression | design.md Invariants / Key Flows | VAL-refresh-integration | integration | sfo-account-api-integration | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-sfo-account-published-source | normal | yes | VAL-src | integration | covered | 全目标编译闭环命中 registry 0.2.1 |
| fh-sfo-account-published-source | boundary | yes | VAL-config-unit | unit | covered | 32 字节 SQL 密钥边界 |
| fh-sfo-account-published-source | negative | yes | VAL-config-unit | unit | covered | 31 字节短 key 拒绝 |
| fh-sfo-account-published-source | error | yes | VAL-config-unit | unit | covered | 校验错误不 panic、不回显密钥 |
| fh-sfo-account-published-source | compatibility | no | VAL-src | contract | not-applicable | 依赖来源切换属迁移面，仅要求编译闭环与锁文件正确 |
| fh-sfo-account-published-source | lifecycle | no | VAL-config-unit | unit | not-applicable | 来源切换无运行时生命周期状态 |
| fh-sfo-account-published-source | cross-module | yes | VAL-src | integration | covered | server + cli workspace 消费方编译 |
| fh-sfo-account-conformance | normal | yes | VAL-login-integration | integration | covered | 登录成功 err=0 与 32 字节 key 启动 |
| fh-sfo-account-conformance | boundary | yes | VAL-config-unit | unit | covered | 32 字节密钥边界 |
| fh-sfo-account-conformance | negative | yes | VAL-login-integration | integration | covered | 未知账号 err=9、限流 err=11 |
| fh-sfo-account-conformance | error | yes | VAL-login-integration | integration | covered | 密码错误 err=10 与英文消息 |
| fh-sfo-account-conformance | compatibility | yes | VAL-login-integration | integration | covered | v1-contract 新语义 + cli e2e 消费闭环 |
| fh-sfo-account-conformance | lifecycle | yes | VAL-dv | dv | covered | 登录/会话/项目主流程 |
| fh-sfo-account-conformance | cross-module | yes | VAL-cli-e2e | integration | covered | filehub-cli e2e 会话 |
| fh-sfo-account-regression | normal | yes | VAL-dv | dv | covered | 主流程回归 |
| fh-sfo-account-regression | boundary | yes | VAL-config-unit | unit | covered | 32 字节边界 |
| fh-sfo-account-regression | negative | yes | VAL-refresh-regression | integration | covered | refresh 不可访问用户接口 |
| fh-sfo-account-regression | error | yes | VAL-login-integration | integration | covered | 登录失败错误类别 |
| fh-sfo-account-regression | compatibility | yes | VAL-cli-e2e | integration | covered | cli 消费兼容 |
| fh-sfo-account-regression | lifecycle | yes | VAL-refresh-regression | integration | covered | 登录 -> refresh 换发 -> 新 session 可用 |
| fh-sfo-account-regression | cross-module | yes | VAL-src | integration | covered | workspace 全目标编译闭环 |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | design/account-dependency.md（session_key <32 / >=32 域） | 31B 拒绝、32B 通过 | unit | covered | not-applicable |
| parameter-domain | design.md（未知/密码错误/限流错误码域） | err=9、err=10、err=11 | integration | covered | not-applicable |
| state-transition | design.md State and Ownership（短 key 启动拒绝 / 长 key 启动） | validate 拒短放长（dv 主流程见 DV Tests） | unit | covered | not-applicable |
| failure-path | design.md Key Flows（短 key 启动失败不 panic） | validate 错误传播断言 | unit | covered | not-applicable |
| error-handling | design.md（9/10/11 错误类别与英文消息） | 三个错误码 HTTP 断言 | integration | covered | not-applicable |
| invariant | design.md Invariants（200 信封、refresh-only） | 登录成功 err=0；refresh 访问拒绝 | integration | covered | not-applicable |
| concurrency | design.md（无新增并发/共享状态声明） | not-applicable: 本任务不引入并发、重入或顺序边界 | unit | not-applicable | 无对应派生用例 |

## Validation Rationale

| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| registry 0.2.1 编译与解析 | workspace `cargo test --no-run --all-targets` + Cargo.lock 检查 | 编译闭环证明 server/cli 消费面兼容 | not-applicable |
| 短 session_key 启动失败 | unit 断言 validate 拒绝 31B、接受 32B 且不回显密钥 | 覆盖 0.2.1 HMAC 下限的两侧分支 | not-applicable |
| 登录失败语义 | 集成断言 err=9/10/11 与英文消息 | 覆盖 v1 契约新语义与枚举信息面边界 | not-applicable |
| refresh-only 与既有会话 | 既有 unit/integration refresh 断言 | 防依赖切换误伤会话边界 | not-applicable |

## Unit Tests

| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| `UsersConfig::validate` | `session_key.len() < 32` | 返回错误且不回显密钥 | server/tests/unit/config.rs | covered | not-applicable |
| `UsersConfig::validate` | `session_key.len() >= 32` | 通过 | server/tests/unit/config.rs | covered | not-applicable |
| `DefaultAccountManager::login`（经 AccountModule） | 未知账号 | `InvalidAccount` + `account ghost not found` | server/tests/unit/account.rs | covered | not-applicable |
| `DefaultAccountManager::login`（经 AccountModule） | 密码错误 | `InvalidPassword` + `Invalid username or password` | server/tests/unit/account.rs | covered | not-applicable |
| `DefaultAccountManager::decode_session` | refresh token | SessionInvalid（既有回归） | server/tests/unit/account.rs | covered | not-applicable |

## DV Tests

| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| 32 字节 key 启动 + 登录/会话/项目主流程 | main | dv_full_workflow_with_tokens_and_gc | 主流程全绿 | server/tests/dv_tests.rs | covered | not-applicable |
| 登录/session/token 生命周期 | lifecycle | dv_full_workflow | 既有全绿 | server/tests/dv_tests.rs | covered | not-applicable |
| 无效凭据失败路径 | failure | dv 既有失败断言 | 拒绝语义保持 | server/tests/dv_tests.rs | covered | not-applicable |

## Integration Tests

| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| `POST /account/login` 错误信封 | sfo-account + account + http | err=0 登录成功 | 未知 err=9、密码 err=10、限流 err=11（英文消息） | server/tests/api_integration.rs | covered | not-applicable |
| `/api/v1/projects` + Bearer | http + account + permissions | session 200 | refresh token 401 | server/tests/api_integration.rs | covered | not-applicable |
| `/account/refresh_session` | sfo-account | refresh 换发 err=0，新 session 可用 | 普通 session 不可换发 | server/tests/api_integration.rs | covered | not-applicable |
| cli e2e（32 字节 fixture） | filehub-cli + server | e2e 登录/发布/下载 | 无 | cli/tests/e2e_cli_server.rs | covered | not-applicable |
| workspace 编译闭环 | server/cli + registry 0.2.1 | 全目标编译通过 | 无 | 任务 testplan contract-compile-closure | covered | not-applicable |

## Definition of Done

- [x] Testing docs 覆盖全部三个 change_id 与受影响子模块（model/config、account、http、cli e2e）
- [x] `testplan.yaml` 与 `testing.md` 引用一致，unit/dv/integration + contract 均注册
- [x] 新测试位于专用测试文件；任务级 `test-run.py filehub/054-switch-published-sfo-account all` 全绿
- [x] 每个实现 change_id 有直接验证与用例类型覆盖
