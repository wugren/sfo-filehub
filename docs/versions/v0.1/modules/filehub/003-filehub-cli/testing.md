---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-20
---

## Approval Record

- approver: user
- approval_date: 2026-08-20
- user_statement: 自动完成003任务吧

# filehub 发布客户端（filehub-cli）测试设计

Risk profile: ./risk-profile.yaml

## Test Document Index
| Document | Topic | Scope |
|----------|-------|-------|
| `cli/src/` 内 `#[cfg(test)]`（lib 单测） | 单元测试：credential_store、archive、login 参数域 | 凭据模型/服务器解析、文件名净化/保留名、登录输入优先级与互斥 |
| `cli/tests/dv_tests.rs` | DV：主工作流、失败工作流、配置持久化、安全打包反例 | 整 crate 服务边界与生命周期 |
| `cli/tests/api_integration.rs` | 集成：进程内 mock v1 服务契约 | v1 JSON/错误映射、multipart、下载流、401 续期重试、token/session 边界 |

## Unified Test Entry
- 机器化任务计划：`docs/versions/v0.1/modules/filehub/003-filehub-cli/testplan.yaml`
- 任务 all：`UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/003-filehub-cli all`
- 单任务边界：只选 `<module>/<task-name>`，不跑模块级/`all all`。
- 所有生成测试均通过 `harness/scripts/test-run.py` 可达。

## Submodule Tests
| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| cli（装配） | 命令解析、登录参数域、退出码 | 本文件 + `cli/src/cli/login_handler.rs` tests | 显式选项 > 环境变量 > 交互提示；两模式互斥；非终端须显式模式 | 互斥冲突、无凭据、stdin 非终端、空密码/用户名 | unit | `cli/src/cli/login_handler.rs` | covered | not-applicable |
| apiclient（技术） | v1 传输、错误映射、401 续期一次 | `cli/tests/api_integration.rs` | sfo-http 包装解析、Bearer 注入、409/403/404/422 映射、流式下载 | 401 续期一次、token 401 不续期、读取失败 | integration | `cli/tests/api_integration.rs` | covered | not-applicable |
| credential_store（技术） | 本地凭据独占、原子写/最小权限、token > session | `cli/src/credential_store/mod.rs` tests + `cli/tests/dv_tests.rs` | 服务器解析优先级、login 覆盖互斥、logout 清除、损坏文件不覆盖 | 损坏 TOML、无服务器、未登录、token/session 互斥 | unit / dv | `cli/src/credential_store/mod.rs`、`cli/tests/dv_tests.rs` | covered | not-applicable |
| archive（技术） | 安全打包、文件名净化、SHA-256 校验后落盘 | `cli/src/archive/mod.rs` tests + `cli/tests/dv_tests.rs` | 目录/单文件打包、越界符号链接拒绝、净化名、校验失败不落盘 | 越界/失效符号链接、超长名、Windows 保留名、SHA 不一致 | unit / dv / integration | `cli/src/archive/mod.rs`、`cli/tests/dv_tests.rs`、`cli/tests/api_integration.rs` | covered | not-applicable |

## Module-Level Tests
| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| 登录凭据持久化与 logout 清除 | credential_store 生命周期 | dv_full_publish_download_versions_workflow + api login/logout | 重新 open 可读 session；logout 后无凭据 | unit / dv / integration | CLI 测试套件 | covered | not-applicable |
| 主工作流：login -> publish -> download -> versions | 四条命令面全链路 | dv_full_publish_download_versions_workflow | 发布 0、下载字节与 mock 载荷一致、版本列表含 v1/v2 | dv | cli/tests/dv_tests.rs | covered | not-applicable |
| 失败工作流：未登录/409 | 退出码 2/4 | dv_no_credential_and_409_failure_workflows | 未登录 exit 2；重复版本 exit 4 | dv | cli/tests/dv_tests.rs | covered | not-applicable |
| 配置损坏不覆盖 | 安全边界 | dv_corrupt_config_is_not_overwritten | 解析失败 exit 8，文件原样保留 | dv | cli/tests/dv_tests.rs | covered | not-applicable |
| 版本输出文本/JSON | 脚本输出契约 | versions_outputs_json_and_text_files | JSON 数组为 VersionDto 字段；text 固定列 | integration | cli/tests/api_integration.rs | covered | not-applicable |
| 下载最新/指定版本 | latest 语义 | versions_latest_and_none_input_targets | 省略版本解析为 latest（v2） | integration | cli/tests/api_integration.rs | covered | not-applicable |

## External Interface Tests
| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| POST /account/login | 密码登录 | 正确密码 200 并保存 session | 错误密码 401 不写凭据；模式互斥 | integration | cli/tests/api_integration.rs | covered | not-applicable |
| POST /account/refresh_session | session 续期 | s1 -> s2 续期并落盘 | 401 不续期/非法 refresh | integration | cli/tests/api_integration.rs | covered | not-applicable |
| GET /api/v1/projects | token 校验与项目解析 | token 有效 200 | token 无效 401 不写凭据 | integration | cli/tests/api_integration.rs | covered | not-applicable |
| POST /api/v1/projects/{id}/versions | multipart 发布 | 201 成功 | 409 冲突；403 只读 token；422 缺字段 | integration | cli/tests/api_integration.rs | covered | not-applicable |
| GET /api/v1/projects/{id}/versions/{v}/download | 流式下载 | 200 gzip 流 + SHA-256 校验 | 内容损坏 exit 7；下载流中 401 不部分重试 | integration | cli/tests/api_integration.rs | covered | not-applicable |

## Direct Change Coverage
| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-cli-login | design.md Directly Mapped Change Items + design/credential-store.md + design/cli.md | VAL-cli-login | integration | integration-cargo | no | not-applicable |
| fh-cli-publish | design.md Directly Mapped Change Items + design/archive.md + design/apiclient.md | VAL-cli-publish | integration | integration-cargo | no | not-applicable |
| fh-cli-download | design.md Directly Mapped Change Items + design/archive.md + design/apiclient.md | VAL-cli-download | integration | integration-cargo | no | not-applicable |
| fh-cli-versions | design.md Directly Mapped Change Items + design/apiclient.md + design/cli.md | VAL-cli-versions | integration | integration-cargo | no | not-applicable |

## Case-Type Coverage
| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-cli-login | normal | yes | VAL-login-normal | integration | covered | not-applicable |
| fh-cli-login | boundary | yes | VAL-login-boundary | unit | covered | not-applicable |
| fh-cli-login | negative | yes | VAL-login-negative | integration | covered | not-applicable |
| fh-cli-login | error | yes | VAL-login-error | dv | covered | not-applicable |
| fh-cli-login | compatibility | yes | VAL-login-compatibility | unit | covered | not-applicable |
| fh-cli-login | lifecycle | yes | VAL-login-lifecycle | dv | covered | not-applicable |
| fh-cli-login | cross-module | yes | VAL-login-cross-module | integration | covered | not-applicable |
| fh-cli-publish | normal | yes | VAL-publish-normal | integration | covered | not-applicable |
| fh-cli-publish | boundary | yes | VAL-publish-boundary | dv | covered | not-applicable |
| fh-cli-publish | negative | yes | VAL-publish-negative | integration | covered | not-applicable |
| fh-cli-publish | error | yes | VAL-publish-error | dv | covered | not-applicable |
| fh-cli-publish | compatibility | yes | VAL-publish-compatibility | unit | covered | not-applicable |
| fh-cli-publish | lifecycle | yes | VAL-publish-lifecycle | dv | covered | not-applicable |
| fh-cli-publish | cross-module | yes | VAL-publish-cross-module | integration | covered | not-applicable |
| fh-cli-download | normal | yes | VAL-download-normal | integration | covered | not-applicable |
| fh-cli-download | boundary | yes | VAL-download-boundary | integration | covered | not-applicable |
| fh-cli-download | negative | yes | VAL-download-negative | integration | covered | not-applicable |
| fh-cli-download | error | yes | VAL-download-error | integration | covered | not-applicable |
| fh-cli-download | compatibility | yes | VAL-download-compatibility | unit | covered | not-applicable |
| fh-cli-download | lifecycle | yes | VAL-download-lifecycle | dv | covered | not-applicable |
| fh-cli-download | cross-module | yes | VAL-download-cross-module | integration | covered | not-applicable |
| fh-cli-versions | normal | yes | VAL-versions-normal | integration | covered | not-applicable |
| fh-cli-versions | boundary | yes | VAL-versions-boundary | integration | covered | not-applicable |
| fh-cli-versions | negative | yes | VAL-versions-negative | integration | covered | not-applicable |
| fh-cli-versions | error | yes | VAL-versions-error | integration | covered | not-applicable |
| fh-cli-versions | compatibility | yes | VAL-versions-compatibility | unit | covered | not-applicable |
| fh-cli-versions | lifecycle | yes | VAL-versions-lifecycle | dv | covered | not-applicable |
| fh-cli-versions | cross-module | yes | VAL-versions-cross-module | integration | covered | not-applicable |

## Design Element Coverage
| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | design/cli.md 命令面/参数 + design/archive.md 文件名净化 | VAL-login-boundary、VAL-publish-boundary、VAL-download-boundary、VAL-versions-boundary、sanitize 超长/保留名 | unit | covered | not-applicable |
| state-transition | design/credential-store.md 凭据状态机 | VAL-login-lifecycle（save_session/save_token/logout/update_session 互斥与清除） | unit | covered | not-applicable |
| failure-path | design.md Key Flows 发布/下载失败 | VAL-publish-error（409）、VAL-download-error（SHA 不一致/部分下载 401 不重试） | integration | covered | not-applicable |
| error-handling | design/apiclient.md 错误映射 + design/cli.md 退出码表 | VAL-login-negative、VAL-versions-negative（401/404）、403/422 映射 | integration | covered | not-applicable |
| invariant | design.md Invariants（token > session、互斥、凭据不落日志、SHA 校验后落盘） | VAL-login-compatibility、VAL-download-normal、corrupt 配置不覆盖 | dv | covered | not-applicable |
| concurrency | design.md 401 续期一次/重试边界 | VAL-login-cross-module、VAL-download-cross-module（续期一次后重试） | integration | covered | not-applicable |

## Validation Rationale
| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| 凭据安全：明文不入参数/日志、最小权限、失败不写凭据 | 错误密码/无效 token 后配置文件无凭据；损坏配置 exit 8 且原文件保留 | 覆盖「不写凭据」与「不覆盖损坏配置」两个不可回滚分支 | not-applicable |
| token > session 复用与互斥 | credential_store 单测断言 save_token 清 session、save_session 清 token | 内存模型层直接证明互斥不变量 | not-applicable |
| 401 续期只一次 | 下载流中 401 一次后 refresh 到 s2 成功；run_auth 续期后重试成功 | 正反路径都验证了「一次」边界 | not-applicable |
| 归档安全 | 越界符号链接拒绝、同树符号链接允许、净化名防穿越 | 在解包/路径语义层断言安全裁剪 | not-applicable |
| 下载幂等与完整性 | 下载字节与 mock 载荷一致、SHA 校验失败 exit 7 且不落盘 | 端到端验证校验后 rename 语义 | not-applicable |
| 跨模块契约 | 进程内 mock 按 `docs/api/v1-contract.md` 形状返回 | DTO 字段、错误体、multipart 形状与真实服务端实现一致 | not-applicable |

## Unit Tests
| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| CredentialStore::current_credential | token 存在 / 仅 session | token 优先；save_token 清 session；save_session 清 token | cli/src/credential_store/mod.rs | covered | not-applicable |
| CredentialStore::resolve_server | 显式 > env > default > 唯一已存 | URL 规范化与优先级 | cli/src/credential_store/mod.rs | covered | not-applicable |
| sanitize_artifact_name | 非法字符/超长/保留名 | 仅保留安全字符、长度 ≤255、CON/NUL 前缀转义 | cli/src/archive/mod.rs | covered | not-applicable |
| collect_login_inputs | 非终端无模式 / env token | stdin 非终端须显式模式；FILEHUB_TOKEN 通道生效 | cli/src/cli/login_handler.rs | covered | not-applicable |

## DV Tests
| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| 主工作流 | main | dv_full_publish_download_versions_workflow | 登录 -> 发布 -> 下载(SHA 一致) -> 版本列表 -> logout | cli/tests/dv_tests.rs | covered | not-applicable |
| 凭据生命周期 | lifecycle | dv_full_publish_download_versions_workflow | login 写入凭据 -> logout 清空 -> re-login 重新写入 | cli/tests/dv_tests.rs | covered | not-applicable |
| 失败工作流 | failure | dv_no_credential_and_409_failure_workflows | 未登录 exit 2；409 exit 4 | cli/tests/dv_tests.rs | covered | not-applicable |
| 安全打包反例 | failure | dv_unsafe_symlink_archive_is_rejected | 越界符号链接拒绝；同树链接允许 | cli/tests/dv_tests.rs | covered | not-applicable |
| 配置持久化/损坏 | persistence | dv_corrupt_config_is_not_overwritten | 损坏配置 exit 8 且文件不变 | cli/tests/dv_tests.rs | covered | not-applicable |

## Integration Tests
| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| 密码/token 登录 | cli + apiclient + credential_store | sfo-http 包装解析成功并持久化 | 错误密码/无效 token 不写凭据 | cli/tests/api_integration.rs | covered | not-applicable |
| 401 续期重试 | cli + apiclient + credential_store | run_auth/download 遇 401 续期一次后成功 | token 401 不续期直接失败；下载流部分字节不重试 | cli/tests/api_integration.rs | covered | not-applicable |
| 发布冲突与越权 | cli + apiclient | 201 成功 | 409 conflict exit 4；403 forbidden exit 3 | cli/tests/api_integration.rs | covered | not-applicable |
| 下载完整性 | cli + archive + apiclient | 字节、SHA 一致且净化名落盘 | 损坏流 exit 7、临时文件清理 | cli/tests/api_integration.rs | covered | not-applicable |
| 版本查询输出 | cli + apiclient | JSON/文本输出到文件 | 404 not_found 映射为输入无效 | cli/tests/api_integration.rs | covered | not-applicable |

## Regression Focus
- 高风险边界：401 最多续期一次且续期失败不重试；token 401 不续期；下载流开始后不部分重试；损坏配置不覆盖；凭据失败不写盘；越界符号链接拒绝。
- 契约对齐：sfo-http 包装 `{err,result}`、统一错误体 `{error,message}`、multipart 字段 `version/file/sha256`、`.tar.gz` 下载流——与 `docs/api/v1-contract.md` 及 001 服务端实现一致。

## Definition of Done
- [x] Testing docs 覆盖全部四个实现 change_id 与 7 类用例
- [x] `testplan.yaml` 与 `testing.md` 引用一致
- [x] 新测试均注册在 `harness/scripts/test-run.py` 可达的任务 testplan
- [x] 单元测试位于 `cli/src/` `#[cfg(test)]`；DV 位于 `cli/tests/dv_tests.rs`；集成位于 `cli/tests/api_integration.rs`
- [x] `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/003-filehub-cli all` 只包含本任务步骤
- [x] 每个实现 change_id 均有直接验证与用例类型覆盖
- [x] 相关自动化测试已通过
