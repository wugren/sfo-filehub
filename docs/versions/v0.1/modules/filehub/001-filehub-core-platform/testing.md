---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-19
---

# filehub 服务后台（filehub-server）测试设计

Risk profile: ./risk-profile.yaml

## Test Document Index
| Document | Topic | Scope |
|----------|-------|-------|
| `tests/unit_tests.rs` + `tests/unit/` | 单元测试：模型、账号、权限、token、文件、版本、项目 | 各子模块函数/分支验证 |
| `tests/dv_tests.rs` | DV：单模块生命周期、主工作流、失败工作流、配置变体、持久化 | 整 crate 服务边界 |
| `tests/api_integration.rs` | 集成：Actix/sfo-http 真实 HTTP 契约 | v1 API 与跨模块数据流 |

## Unified Test Entry
- 机器化任务计划：`docs/versions/v0.1/modules/filehub/001-filehub-core-platform/testplan.yaml`
- 任务 all：`UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/001-filehub-core-platform all`
- 单任务边界：只选 `<module>/<task-name>`，不跑模块级/`all all`。
- 所有生成测试均通过 `harness/scripts/test-run.py` 可达。

## Submodule Tests
| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| account | sfo-account 装配/初始化 | tests/unit/account.rs | 初始账号、登录、会话解码、refresh、幂等 | 错误口令、垃圾凭据 | unit | tests/unit/account.rs | covered | not-applicable |
| permissions | 统一权限判定 | tests/unit/permissions.rs | 任意账号可创建、项目矩阵、协作者、token 二次限制 | 匿名/越权/owner 保护 | unit | tests/unit/permissions.rs | covered | not-applicable |
| tokens | token 生命周期与 JWT | tests/unit/tokens.rs | 创建/列表/改名/重签/轮换/撤销/过期校验 | 旧 JWT 失效、超过 1 年、他人 token | unit | tests/unit/tokens.rs | covered | not-applicable |
| storage | .tar.gz 存储 | tests/unit/storage.rs | ingest/open/discard/gc、SHA、格式与上限 | 非 gzip、SHA 不一致、孤儿清理 | unit | tests/unit/storage.rs | covered | not-applicable |
| versions | 版本/发布协调 | tests/unit/versions.rs | publish/list/latest/referenced、权限落库 | 409、只读拒绝 | unit | tests/unit/versions.rs | covered | not-applicable |
| projects | 项目 CRUD/可见性 | tests/unit/projects.rs | create/list/visibility/delete | 任意账号可创建、重名、非 owner 无删除权 | unit | tests/unit/projects.rs | covered | not-applicable |
| http/contract | 路由/DTO/错误映射 | tests/api_integration.rs | v1 HTTP 全链路 | 401/403/404/409/422 | integration | tests/api_integration.rs | covered | not-applicable |

## Module-Level Tests
| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| 账号与会话 | 登录/解码/refresh/幂等初始化 | login_session + decode_session | 正反例通过 | unit | tests/unit/account.rs | covered | not-applicable |
| 权限矩阵 | 账号 x 项目 x token 二次限制 | can_access | 冻结矩阵全部断言通过 | unit | tests/unit/permissions.rs | covered | not-applicable |
| token 生命周期 | create/update/rotate/revoke/resolve | TokenService | 旧凭据即时失效 | unit | tests/unit/tokens.rs | covered | not-applicable |
| 文件原子性 | ingest/discard/gc | FileStore | 无半成品/孤儿 | unit | tests/unit/storage.rs | covered | not-applicable |
| 版本不可覆盖 | unique(latest) | VersionService | 409 + latest | unit | tests/unit/versions.rs | covered | not-applicable |
| 项目可见性 | public/private | ProjectService | 匿名只读/私有强制授权 | unit | tests/unit/projects.rs | covered | not-applicable |

## External Interface Tests
| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| /account/*（sfo-account） | 登录/会话/refresh | login + get_account_info | 凭据无效 | integration | tests/api_integration.rs | covered | not-applicable |
| /api/v1/projects* | 项目/协作者 | create/list/visibility/collaborators | 401/403/404/409 | integration | tests/api_integration.rs | covered | not-applicable |
| /api/v1/tokens* | token 管理 | create/list/rotate/revoke | 422、旧 token 401 | integration | tests/api_integration.rs | covered | not-applicable |
| /api/v1/projects/<built-in function id>/versions* | 发布/列表/下载 | 201/200/下载 SHA | 409/401/403/422 | integration | tests/api_integration.rs | covered | not-applicable |

## Direct Change Coverage
| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-server-account | `design.md` Directly Mapped Change Items + `design/account.md` | VAL-account-coverage | integration | integration-api | no | not-applicable |
| fh-server-permissions | `design.md` Directly Mapped Change Items + `design/permissions.md` | VAL-permissions-coverage | integration | integration-api | no | not-applicable |
| fh-server-tokens | `design.md` Directly Mapped Change Items + `design/tokens.md` | VAL-tokens-coverage | integration | integration-api | no | not-applicable |
| fh-server-files | `design.md` Directly Mapped Change Items + `design/storage.md` | VAL-storage-coverage | integration | integration-api | no | not-applicable |
| fh-server-versions | `design.md` Directly Mapped Change Items + `design/versions.md` | VAL-versions-coverage | integration | integration-api | no | not-applicable |
| fh-server-projects | `design.md` Directly Mapped Change Items + `design/projects.md` | VAL-projects-coverage | integration | integration-api | no | not-applicable |
| fh-server-http | `design.md` Directly Mapped Change Items + `design/model.md` | VAL-http-coverage | integration | integration-api | no | not-applicable |

## Case-Type Coverage
| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-server-account | normal | yes | VAL-account-normal | integration | covered | not-applicable |
| fh-server-account | boundary | yes | VAL-account-boundary | unit | covered | not-applicable |
| fh-server-account | negative | yes | VAL-account-negative | integration | covered | not-applicable |
| fh-server-account | error | yes | VAL-account-error | integration | covered | not-applicable |
| fh-server-account | compatibility | yes | VAL-account-compatibility | unit | covered | not-applicable |
| fh-server-account | lifecycle | yes | VAL-account-lifecycle | dv | covered | not-applicable |
| fh-server-account | cross-module | yes | VAL-account-cross-module | integration | covered | not-applicable |
| fh-server-permissions | normal | yes | VAL-permissions-normal | integration | covered | not-applicable |
| fh-server-permissions | boundary | yes | VAL-permissions-boundary | unit | covered | not-applicable |
| fh-server-permissions | negative | yes | VAL-permissions-negative | integration | covered | not-applicable |
| fh-server-permissions | error | yes | VAL-permissions-error | integration | covered | not-applicable |
| fh-server-permissions | compatibility | yes | VAL-permissions-compatibility | unit | covered | not-applicable |
| fh-server-permissions | lifecycle | yes | VAL-permissions-lifecycle | dv | covered | not-applicable |
| fh-server-permissions | cross-module | yes | VAL-permissions-cross-module | integration | covered | not-applicable |
| fh-server-tokens | normal | yes | VAL-tokens-normal | integration | covered | not-applicable |
| fh-server-tokens | boundary | yes | VAL-tokens-boundary | unit | covered | not-applicable |
| fh-server-tokens | negative | yes | VAL-tokens-negative | integration | covered | not-applicable |
| fh-server-tokens | error | yes | VAL-tokens-error | integration | covered | not-applicable |
| fh-server-tokens | compatibility | yes | VAL-tokens-compatibility | unit | covered | not-applicable |
| fh-server-tokens | lifecycle | yes | VAL-tokens-lifecycle | dv | covered | not-applicable |
| fh-server-tokens | cross-module | yes | VAL-tokens-cross-module | integration | covered | not-applicable |
| fh-server-files | normal | yes | VAL-storage-normal | integration | covered | not-applicable |
| fh-server-files | boundary | yes | VAL-storage-boundary | unit | covered | not-applicable |
| fh-server-files | negative | yes | VAL-storage-negative | integration | covered | not-applicable |
| fh-server-files | error | yes | VAL-storage-error | integration | covered | not-applicable |
| fh-server-files | compatibility | yes | VAL-storage-compatibility | unit | covered | not-applicable |
| fh-server-files | lifecycle | yes | VAL-storage-lifecycle | dv | covered | not-applicable |
| fh-server-files | cross-module | yes | VAL-storage-cross-module | integration | covered | not-applicable |
| fh-server-versions | normal | yes | VAL-versions-normal | integration | covered | not-applicable |
| fh-server-versions | boundary | yes | VAL-versions-boundary | unit | covered | not-applicable |
| fh-server-versions | negative | yes | VAL-versions-negative | integration | covered | not-applicable |
| fh-server-versions | error | yes | VAL-versions-error | integration | covered | not-applicable |
| fh-server-versions | compatibility | yes | VAL-versions-compatibility | unit | covered | not-applicable |
| fh-server-versions | lifecycle | yes | VAL-versions-lifecycle | dv | covered | not-applicable |
| fh-server-versions | cross-module | yes | VAL-versions-cross-module | integration | covered | not-applicable |
| fh-server-projects | normal | yes | VAL-projects-normal | integration | covered | not-applicable |
| fh-server-projects | boundary | yes | VAL-projects-boundary | unit | covered | not-applicable |
| fh-server-projects | negative | yes | VAL-projects-negative | integration | covered | not-applicable |
| fh-server-projects | error | yes | VAL-projects-error | integration | covered | not-applicable |
| fh-server-projects | compatibility | yes | VAL-projects-compatibility | unit | covered | not-applicable |
| fh-server-projects | lifecycle | yes | VAL-projects-lifecycle | dv | covered | not-applicable |
| fh-server-projects | cross-module | yes | VAL-projects-cross-module | integration | covered | not-applicable |
| fh-server-http | normal | yes | VAL-http-normal | integration | covered | not-applicable |
| fh-server-http | boundary | yes | VAL-http-boundary | unit | covered | not-applicable |
| fh-server-http | negative | yes | VAL-http-negative | integration | covered | not-applicable |
| fh-server-http | error | yes | VAL-http-error | integration | covered | not-applicable |
| fh-server-http | compatibility | yes | VAL-http-compatibility | unit | covered | not-applicable |
| fh-server-http | lifecycle | yes | VAL-http-lifecycle | dv | covered | not-applicable |
| fh-server-http | cross-module | yes | VAL-http-cross-module | integration | covered | not-applicable |

## Design Element Coverage
| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | `design.md` File-Level Interfaces + `design/model.md`、`design/tokens.md` | VAL-model-scope/role/visibility、VAL-token-expiry 边界、VAL-upload-size | unit | covered | not-applicable |
| state-transition | `design/tokens.md` State and Ownership（create/update/rotate/revoke） | VAL-token-lifecycle、VAL-rename-no-resign、VAL-rotate-invalidates-old | unit | covered | not-applicable |
| failure-path | `design.md` Key Flows 发布失败/409、`design/files.md` 原子写入失败 | VAL-publish-conflict、VAL-discard-after-failure、VAL-gc-orphans | dv | covered | not-applicable |
| error-handling | `design/http.md` 错误映射 + `design/versions.md` Forbidden/Conflict | VAL-unauthorized-private、VAL-missing-project、VAL-sha-mismatch、VAL-invalid-expiry | integration | covered | not-applicable |
| invariant | `design.md` Invariants（版本唯一、SHA 一致、路径防穿越、token<=用户） | VAL-version-unique、VAL-download-sha、VAL-token-second-limit、VAL-safe-path | integration | covered | not-applicable |
| concurrency | `design/tokens.md` rotate/revoke 并发与旧 JWT 失效声明 | VAL-rotate-revokes-old、VAL-revoke-invalidates | unit | covered | not-applicable |

## Validation Rationale
| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| 认证/session 与 token 凭据不互冒 | account/token 正反例 + HTTP 401 | 两条验签路径独立且都失败时拒绝 | not-applicable |
| 版本不可覆盖与原子发布 | 409 + 下载 SHA | 唯一约束与回滚分支同时被断言 | not-applicable |
| public 匿名只读/private 强制授权 | unit 矩阵 + HTTP 状态 | 三层矩阵与 wire 状态一致 | not-applicable |
| token 权限不超过用户权限 | 二次限制单元 + read-only token 写拒绝 | 服务层与 HTTP 层双保险 | not-applicable |
| 路径防穿越与孤儿回收 | safe_path + gc keep | 实现测试覆盖 rel_path 约束与孤儿清理 | not-applicable |

## Unit Tests
| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| model 枚举/Scope | parse/display round trip 与非法输入 | 角色、权限、可见性序列化往返与拒绝分支 | tests/unit/model.rs | covered | not-applicable |
| AccountModule::init | 首次初始化与幂等重现 | alice/bob 初始账号就绪，重复 assemble 不产生重复用户 | tests/unit/account.rs | covered | not-applicable |
| DefaultAccountManager::login/decode/refresh | 正确/错误口令、会话解码、refresh | 正例成功、garbage 解码失败、错误密码失败 | tests/unit/account.rs | covered | not-applicable |
| FileStore::ingest | tar.gz 校验/SHA/超限分支 | 合法 tar.gz 入库、非 gzip 422、SHA 不一致 422 | tests/unit/storage.rs | covered | not-applicable |
| FileStore::discard/gc_orphans | 已引用与孤儿分支 | 无引用 discard 成功、keep 集合保留引用文件 | tests/unit/storage.rs | covered | not-applicable |
| PermissionChecker::can_access | Anonymous/User/Token 三分支与二次限制 | public 只读、private deny、owner 隐式 admin、token scope 交集 | tests/unit/permissions.rs | covered | not-applicable |
| collaborator grant/update/remove | administration 前置校验 | 协作者列表、owner 不可授权/移除 | tests/unit/permissions.rs | covered | not-applicable |
| TokenService::create/update/rotate/revoke/resolve | 重签/轮换/撤销/过期分支 | 仅 name 不重签、scope 变更旧 JWT 失效、超过 1 年拒绝 | tests/unit/tokens.rs | covered | not-applicable |
| ProjectService::create/list/delete | 任意账号可创建/冲突/可见性/删除权 | 创建者成为 owner、重名冲突、delete 仅项目 owner | tests/unit/projects.rs | covered | not-applicable |
| VersionService::publish/list/get | 版本唯一/latest/只读协作者 | 409 重复、latest 倒序、read 角色不可发布 | tests/unit/versions.rs | covered | not-applicable |

## DV Tests
| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| 生命周期装配与启动 GC | lifecycle | AppState::assemble + startup_gc | 模块可用且无孤儿残留 | tests/dv_tests.rs | covered | not-applicable |
| 主工作流：登录/建项目/发布/公开下载 | main | dv_full_workflow_with_tokens_and_gc | session 登录、owner 建项目、发布两版本、public 匿名读与下载 | tests/dv_tests.rs | covered | not-applicable |
| 失败工作流：越权/重复发布/超限 | failure | dv_full_workflow 内失败断言 + small config | private 拒绝、409、64B 超过 32B 上限被拒 | tests/dv_tests.rs | covered | not-applicable |
| 配置变体：max_archive_bytes | config | small config 实例 | 超限 422 | tests/dv_tests.rs | covered | not-applicable |
| 持久化恢复：SQLite 重开 | persistence | dv_persistence_across_reopen | 项目与版本在重开 pool 后可读 | tests/dv_tests.rs | covered | not-applicable |

## Integration Tests
| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| 登录/会话/refresh HTTP | account + sfo-http | POST /account/login、GET /account/get_account_info | Bearer session 获取当前账号 | tests/api_integration.rs | covered | not-applicable |
| 项目可见性边界 | http + projects + permissions | 任意账号创建 private、匿名 401、切 public 后匿名 200 | 错误状态码断言 | tests/api_integration.rs | covered | not-applicable |
| 版本发布/列表/下载 | http + versions + storage + files | multipart 发布 201、重复 409、latest、下载字节与 SHA 一致 | 409 与下载 SHA 失败断言 | tests/api_integration.rs | covered | not-applicable |
| token 生命周期与二次限制 | http + tokens + permissions | 创建 201、read token 可读、写权限 403、rotate 旧 token 401 | 旧 token 401、写 403 | tests/api_integration.rs | covered | not-applicable |
| 协作者授权 | http + permissions | grant read 后 bob 可读、bob 写 403 | 未授权/低权限拒绝 | tests/api_integration.rs | covered | not-applicable |

## Regression Focus
- 历史高风险边界：token 无过期字段（JWT exp 唯一承载）、rotate 后旧 JWT 立即失效、项目删除后版本引用不可见、孤儿文件由启动 GC 回收。
- 本次实现补充：`scope` 序列化使用 `metadata:read` 等冒号命名；sfo-http Actix 后端 PATCH 不支持，更新语义端点以 POST 提供并写入契约。

## Definition of Done
- [x] Testing docs 覆盖全部 7 个直接子模块（含共享 model 说明）
- [x] `testplan.yaml` 与 `testing.md` 引用一致
- [x] 新测试均注册在 `harness/scripts/test-run.py` 可达的任务 testplan
- [x] 单元测试位于 `tests/unit/`、DV 位于 `tests/dv_tests.rs`、集成位于 `tests/api_integration.rs`
- [x] `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/001-filehub-core-platform all` 只包含本任务步骤
- [x] 本任务不直接选择模块级/`all all`/root 快捷入口/质量门禁
- [x] 每个实现 change_id 均有直接验证与用例类型覆盖
- [x] 相关自动化测试已通过
