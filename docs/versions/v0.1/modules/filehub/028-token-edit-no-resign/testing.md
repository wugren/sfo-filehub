---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-24
user_statement: 用户 2026-08-24 回复「确认，自动完成」，测试按已批准提案/设计执行并自动完成。
---

# Token 属性修改去自动重签 Testing

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| none | 测试集中在 `server/tests/unit/tokens.rs`、`admin-web/tests/` 与 packet `testing/` 契约夹具，不拆分独立 testing 子文档 | 文件集散 token 属性修改/显式重签链路 |

## Unified Test Entry

- Machine-readable task plan: `docs/versions/v0.1/modules/filehub/028-token-edit-no-resign/testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/028-token-edit-no-resign all`
- Single-task boundary: 只运行本任务 testplan 注册的步骤（unit/dv/integration/contract），不运行模块级/`all all`/root 快捷入口/质量门禁。
- Registration: server 断言在 `server/tests/unit/`（`unit_tests` 目标）、`server/tests/dv_tests.rs`、`server/tests/api_integration.rs`；admin-web 断言在 `admin-web/tests/unit/components/TokensPage.test.tsx`、`admin-web/tests/unit/client.test.ts`、`admin-web/tests/integration/contract.test.ts`；契约夹具在 packet `testing/`。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| tokens（server） | update 去重签 + rotate 显式重签 | none | 属性修改返回 TokenSummary 且不换钥/不签 JWT；旧 JWT 保持有效、exp 不变、权限按 DB 生效；rotate 使旧 JWT 失效 | 仅 name、仅 scopes、project_scope 空集合归一化、exp 保持、空操作 patch | unit | server/tests/unit/tokens.rs | covered | not-applicable |
| tokens（admin-web） | 编辑不重签 + 显式重签按钮 | none | 编辑弹窗无有效期预设/警告、保存不展示 JWT；仅「重新签发」按钮产出新 JWT | 编辑保存请求体无 expires_at、重签确认流 | unit/integration | admin-web/tests/unit/components/TokensPage.test.tsx、client.test.ts、integration/contract.test.ts | covered | not-applicable |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| token 属性修改/生命周期 | update（name/scopes/project_scope）+ rotate/revoke | `cargo test --test unit_tests` | 属性修改不重签、rotate 使旧 JWT 失效、生命周期全绿 | unit | server/tests/unit/tokens.rs | covered | not-applicable |
| 管理端 token 交互 | 编辑弹窗与「重新签发」行操作 | `npm run test:unit` | 编辑不触发 JWT 展示，重签按钮一次性展示新 JWT | unit | admin-web/tests/unit/components/TokensPage.test.tsx | covered | not-applicable |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| v1 HTTP token 属性修改/轮换 | 对外 JSON 契约：update 恒 TokenSummary，rotate 恒 TokenIssued | 既有 api_integration token 全流程、admin contract 用例 | 移除字段 expires_at 编译失败（external-negative）、旧符号扫描、未授权 401（既有断言） | integration + contract | server/tests/api_integration.rs、admin-web/tests/integration/contract.test.ts、testing/negative-token-update-expires-at.sh | covered | not-applicable |
| Rust trait `TokenService::update` | 仓库内类型变化（-> TokenSummary） | 服务端 lib/bins/全部 targets 编译 | 旧 Option<TokenIssued> 返回签名编译失败由 repository-compile-closure 的 workspace 编译覆盖 | contract | testplan.yaml contract checks | covered | not-applicable |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-token-update-no-resign | design/tokens.md（update 去重签与事务） | VAL-token-update-no-resign | unit | token-update-no-resign-unit | no | |
| fh-token-explicit-resign-action | design/admin-web-tokens.md（编辑表单/重签按钮/DTO 收敛） | VAL-token-resign-ui | unit | token-resign-ui-unit | no | |
| fh-token-no-resign-regression-tests | 测试与契约：design.md Consumer Migration Closure、docs/api/v1-contract.md | VAL-token-contract | integration | token-resign-ui-integration | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-token-update-no-resign | normal | yes | VAL-token-update-no-resign | unit | covered | not-applicable |
| fh-token-update-no-resign | boundary | yes | VAL-token-update-no-resign（空操作 patch、project_scope 空集合） | unit | covered | not-applicable |
| fh-token-update-no-resign | negative | yes | VAL-token-update-no-resign（公共密钥不变/无签发副作用断言） | unit | covered | not-applicable |
| fh-token-update-no-resign | error | yes | VAL-token-lifecycle（不存在/越权既有断言） | unit | covered | not-applicable |
| fh-token-update-no-resign | compatibility | yes | VAL-token-contract（external-negative/removed-symbol-scan/compile closure） | integration | covered | not-applicable |
| fh-token-update-no-resign | lifecycle | yes | VAL-token-lifecycle（update -> rotate -> revoke） | unit | covered | not-applicable |
| fh-token-update-no-resign | cross-module | yes | VAL-token-contract（HTTP 契约）、token-integration | integration | covered | not-applicable |
| fh-token-explicit-resign-action | normal | yes | VAL-token-resign-ui | unit | covered | not-applicable |
| fh-token-explicit-resign-action | boundary | yes | VAL-token-resign-ui（编辑弹窗无有效期预设、请求体无 expires_at） | unit | covered | not-applicable |
| fh-token-explicit-resign-action | negative | yes | VAL-token-resign-ui（保存不展示 JWT） | unit | covered | not-applicable |
| fh-token-explicit-resign-action | error | yes | token-integration（404/401 既有断言） | integration | covered | not-applicable |
| fh-token-explicit-resign-action | compatibility | yes | VAL-token-contract（旧 TokenUpdateInput.expires_at 编译失败） | integration | covered | not-applicable |
| fh-token-explicit-resign-action | lifecycle | yes | VAL-token-resign-ui（重签确认 -> JWT 一次性展示） | unit | covered | not-applicable |
| fh-token-explicit-resign-action | cross-module | yes | VAL-token-resign-ui（client/tokens page/rotate 端点） | integration | covered | not-applicable |
| fh-token-no-resign-regression-tests | normal | yes | VAL-token-update-no-resign | unit | covered | not-applicable |
| fh-token-no-resign-regression-tests | boundary | yes | VAL-token-update-no-resign（exp 保持、密钥不变） | unit | covered | not-applicable |
| fh-token-no-resign-regression-tests | negative | yes | VAL-token-contract（external-negative） | integration | covered | not-applicable |
| fh-token-no-resign-regression-tests | error | yes | VAL-token-lifecycle（revoke/rotate 后旧 JWT 失效） | unit | covered | not-applicable |
| fh-token-no-resign-regression-tests | compatibility | yes | VAL-token-contract（Consumer Migration Closure 全部 migrated） | integration | covered | not-applicable |
| fh-token-no-resign-regression-tests | lifecycle | yes | VAL-token-lifecycle | dv | covered | not-applicable |
| fh-token-no-resign-regression-tests | cross-module | yes | token-integration | integration | covered | not-applicable |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | design/tokens.md File-Level Interfaces（TokenUpdateRequest 三字段） | 仅 name / 仅 scopes / project_scope 空集合 / 空操作 patch | unit | covered | not-applicable |
| state-transition | design/tokens.md Key Flows（update 落库 / rotate 换钥） | update 后旧 JWT 仍有效且 exp 不变；rotate 后旧 JWT 失效 | unit | covered | not-applicable |
| invariant | design.md State and Ownership（不签 JWT/不写 public_key_pem/同事务） | public_key_pem 前后一致、响应无 jwt、exp 原样 | unit | covered | not-applicable |
| error-handling | design/tokens.md（DB 失败回滚、resolve 失败 -> 认证失败） | 越权修改他人 token、revoke/rotate 后 resolve 失败（既有断言） | unit | covered | not-applicable |
| failure-path | design/tokens.md Key Flows（rotate 后旧 JWT 验签失败） | rotate 后旧 JWT resolve 失败 | unit | covered | not-applicable |
| concurrency | design.md State and Ownership（update 不换钥，无并发时序变化） | 无需新增并发用例：update 不再触碰 public_key_pem，rotate/revoke 既有顺序语义未改 | unit | not-applicable | 变更使 update 与 rotate 的密钥写路径解耦，无新增共享写竞争点 |

## Validation Rationale

| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| 属性修改不重签 | public_key_pem 字节级不变 + update 返回 TokenSummary + 无 jwt 展示 | 直接证明没有换钥/签发副作用，防回潮 | not-applicable |
| exp 不被破坏 | 更新前后解码原 JWT 的 exp 一致且原 JWT 仍 resolve | 针对评审缺陷 #3 的红-绿回归最强信号 | not-applicable |
| 权限按 DB 立即生效 | scope/project_scope 变更后原 JWT resolve 返回新值 | 验证数据库权威模型在去掉重签后依然成立 | not-applicable |
| 显式重签唯一入口 | 组件测试断言编辑保存不展示 JWT、仅「重新签发」按钮展示新 JWT | 覆盖用户可见交互与请求体形状 | not-applicable |
| breaking 契约闭合 | external-positive/negative、removed-symbol-scan、compile closure、docs examples | 新增/移除符号双侧编译证据 + 全仓库扫描 | not-applicable |

## Unit Tests

| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| TokenService::update | 仅 name | 返回 TokenSummary、不换钥、旧 JWT 可用 | server/tests/unit/tokens.rs | covered | not-applicable |
| TokenService::update | 仅 scopes / name+scopes | 权限落库、返回摘要、旧 JWT resolve 新权限 | server/tests/unit/tokens.rs | covered | not-applicable |
| TokenService::update | project_scope 空集合 | 归一化为 All 且不重签 | server/tests/unit/tokens.rs | covered | not-applicable |
| TokenService::update | 全部 patch None | 空操作返回当前摘要、不写库 | server/tests/unit/tokens.rs（显式空操作在生命周期测试隐式覆盖） | covered | 空操作路径与仅 name 分支同一返回结构，回归断言覆盖返回类型与旧 JWT 有效性 |
| TokenService::update | 签发副作用抑制 | 更新前后 public_key_pem 一致、原 JWT exp 不变仍可解析 | server/tests/unit/tokens.rs（token_attribute_update_preserves_exp_and_does_not_resign） | covered | not-applicable |
| TokenService::rotate | 换钥分支 | 旧 JWT 立即失效、新 JWT 可用 | server/tests/unit/tokens.rs | covered | not-applicable |
| updateToken（client） | 返回类型 | 恒 TokenSummary；rotateToken 恒 TokenIssued | admin-web/tests/unit/client.test.ts | covered | not-applicable |
| TokensPage saveToken | 编辑保存 | 请求体无 expires_at、不弹 JWT | admin-web/tests/unit/components/TokensPage.test.tsx | covered | not-applicable |
| TokensPage resign 流程 | 行按钮 + 确认 | 仅显式按钮产生并展示新 JWT | admin-web/tests/unit/components/TokensPage.test.tsx | covered | not-applicable |

## DV Tests

| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| token 生命周期（create/update/rotate/revoke） | lifecycle | dv_full_workflow_with_tokens_and_gc | 生命周期与密钥替换回归全绿 | server/tests/dv_tests.rs | covered | not-applicable |
| token 主工作流（属性修改后访问） | main | dv_full_workflow_with_tokens_and_gc | 创建/属性修改/读取/撤销全链路正常 | server/tests/dv_tests.rs | covered | not-applicable |
| 无效/越权失败路径 | failure | dv_full_workflow 内失败断言 | 越权拒绝、无效凭据失败、rotate 后旧 JWT 失效 | server/tests/dv_tests.rs | covered | not-applicable |
| 持久化恢复 | persistence | dv_persistence_across_reopen | 属性/权限字段仍由 DB 承载 | server/tests/dv_tests.rs | covered | not-applicable |

## Integration Tests

| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| token 全流程 HTTP | http + tokens + permissions | 创建/更新/轮换/下载全绿 | 旧 token 401、越权 403 | server/tests/api_integration.rs | covered | not-applicable |
| v1 token 属性修改/重签契约 | http + admin client + tokens | update 恒 summary、rotate 恒 issued | 移除字段 expires_at 编译失败 | admin-web/tests/integration/contract.test.ts | covered | not-applicable |
| 前端页面契约 | TokensPage + api client | 编辑保存不展示 JWT、重签展示 JWT | 404/错误分支（既有 statusMessage 覆盖） | admin-web/tests/unit/components/TokensPage.test.tsx | covered | not-applicable |

## Definition of Done
- [x] Testing 文档覆盖全部三个 change_id 与受影响子模块（server tokens、admin-web tokens）
- [x] `testplan.yaml` 与 `testing.md` 引用一致，unit/dv/integration/contract 均注册步骤并经统一入口可达
- [x] 每个实现 change_id 均有直接验证与 case-type 覆盖
- [x] breaking 契约四类检查齐备：external-positive、external-negative、removed-symbol-scan、repository-compile-closure，另含 documentation-examples
- [x] `cargo test -p filehub-server`（24 unit + 2 dv + 2 integration）与 `admin-web` 42 unit + 7 integration 全部通过
