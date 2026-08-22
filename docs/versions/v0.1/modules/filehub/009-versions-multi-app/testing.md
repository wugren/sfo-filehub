---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认，自动完成


# 版本多应用（versions multi-app）测试设计

Risk profile: ./risk-profile.yaml

## Test Document Index

| 文档/脚本 | 职责 |
|-----------|------|
| `testplan.yaml` | 任务级统一入口：contract_checks + unit/dv/integration 步骤与 change_id 绑定 |
| `testing.md` | 测试设计：层级表、设计元素/案例类型覆盖、验证理由与完成定义 |
| `testing/negative-old-publish.sh` | external-negative 契约检查：旧 `FilehubClient::publish` 必须编译失败 |
| server/cli/admin-web 既有测试文件 | 各层级测试实现（本任务修改/新增部分见下表） |

## Unified Test Entry

本任务全部验证经统一入口执行：

`UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/009-versions-multi-app all`

`harness/scripts/test-run.py` 按 `testplan.yaml` 先执行启用中的 `contract_checks`，再依序执行 unit → dv → integration 全部步骤，并把每次运行写入 `.harness/test-results/test-runs/`。

## Submodule Tests

| design 子模块 | 测试落点 |
|--------------|----------|
| design/versions.md | `server/tests/unit/versions.rs`（服务层）、`server/tests/dv_tests.rs`（生命周期）、`server/tests/unit/storage.rs`（引用/孤儿回收） |
| design/api.md | `server/tests/api_integration.rs`（真实 Actix HTTP）、contract_checks（negative/scan/closure/docs） |
| design/cli.md | `cli/tests/dv_tests.rs`、`cli/tests/api_integration.rs`（mock v1 契约） |
| design/web.md | `admin-web/tests/unit/client.test.ts`、`admin-web/tests/integration/contract.test.ts`、`admin-web/tests/dv/verify-dist.mjs` |

## Module-Level Tests

- filehub-server：`cargo test -p filehub-server`（unit_tests 例数随分支补齐后全绿、dv 1 例、api_integration 3 例全绿，见统一入口 run artifact）。
- filehub-cli：`cargo test -p filehub-cli`（lib 6 例、dv 11 例、api_integration 4 例全绿）。
- filehub-web：`npm run test:unit`（37 例）、`npm run test:integration`（7 例）、`npm run test:dv`（build + dist 校验）全绿。

## External Interface Tests

- v1 HTTP 契约正反例：真实 Actix 服务（`server/tests/api_integration.rs`）、CLI 进程内 mock（`cli/tests/common/mod.rs`）、admin-web stub server（`admin-web/tests/integration/contract.test.ts`）。
- breaking 契约检查（testplan `contract_checks`）：external-positive、external-negative、removed-symbol-scan（`consumer-closure-check.py`）、repository-compile-closure、documentation-examples。

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | gap | gap_manual_reason |
|-----------|--------------|---------------|----------------|------------------|-----|-------------------|
| fh-versions-multi-app-model | design/versions.md | V001/V101/V201（unit/dv 见表） | integration | integration-server | no | - |
| fh-versions-multi-app-api | design/api.md | V201/V204（contract 步骤见表） | integration | integration-server | no | - |
| fh-cli-multi-app | design/cli.md | V003/V102/V202（dv 见表） | integration | integration-cli | no | - |
| fh-web-multi-app | design/web.md | V004/V103/V203（unit/dv 见表） | integration | integration-web | no | - |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-versions-multi-app-model | normal | yes | V001 | unit | covered | - |
| fh-versions-multi-app-model | boundary | yes | V001 | unit | covered | - |
| fh-versions-multi-app-model | negative | yes | V001 | unit | covered | - |
| fh-versions-multi-app-model | error | yes | V001 | unit | covered | - |
| fh-versions-multi-app-model | compatibility | yes | V001 | unit | covered | - |
| fh-versions-multi-app-model | lifecycle | yes | V101 | dv | covered | - |
| fh-versions-multi-app-model | cross-module | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | normal | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | boundary | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | negative | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | error | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | compatibility | yes | V201 | integration | covered | - |
| fh-versions-multi-app-api | lifecycle | yes | V101 | dv | covered | - |
| fh-versions-multi-app-api | cross-module | yes | V202/V203 | integration | covered | - |
| fh-cli-multi-app | normal | yes | V102 | dv | covered | - |
| fh-cli-multi-app | boundary | yes | V102 | dv | covered | - |
| fh-cli-multi-app | negative | yes | V102 | dv | covered | - |
| fh-cli-multi-app | error | yes | V202 | integration | covered | - |
| fh-cli-multi-app | compatibility | yes | V102 | dv | covered | - |
| fh-cli-multi-app | lifecycle | yes | V102 | dv | covered | - |
| fh-cli-multi-app | cross-module | yes | V202 | integration | covered | - |
| fh-web-multi-app | normal | yes | V004 | unit | covered | - |
| fh-web-multi-app | boundary | yes | V004 | unit | covered | - |
| fh-web-multi-app | negative | yes | V203 | integration | covered | - |
| fh-web-multi-app | error | yes | V004 | unit | covered | - |
| fh-web-multi-app | compatibility | yes | V203 | integration | covered | - |
| fh-web-multi-app | lifecycle | yes | V103 | dv | manual | 页面锁定后禁用写操作以 build + 集成 stub 覆盖后端语义；未做组件级渲染断言，见 Unit Tests 表 |
| fh-web-multi-app | cross-module | yes | V203 | integration | covered | - |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | design/versions.md 校验约束 | 空/非法 version、空/非法 app 字符、下载 app None/Some、单 app 缺省下载、多 app 缺省 422 | unit | covered | - |
| state-transition | design/versions.md State and Ownership | Active→Locked、锁定后发布/更新/删除拒绝、重复锁定幂等、空版本查询 | dv | covered | - |
| failure-path | design/api.md Key Flows 失败分支 | 发布到不存在版本 404、文件入库后发布失败 discard、删除不存在 app 404 | unit | covered | - |
| error-handling | design/api.md 错误映射 | NotFound/Conflict/InvalidInput/Forbidden/401 各触发用例 | integration | covered | - |
| invariant | design/versions.md 数据归属 | 重复创建版本 409、同版本多 app 共存、更新后旧文件离开引用集、锁定写入拒绝 | dv | covered | - |
| concurrency | design/versions.md 事务/锁定原子性 | 唯一约束兜底 + 事务内锁定检查：真实并发注入未做 | manual | manual | SQLite 单进程测试未做多连接并发注入；依赖 UNIQUE(project_id,version)/UNIQUE(version_id,app) 与事务内读锁，已有同语义串行用例覆盖 |

## Validation Rationale

- 服务层校验放 unit（分支级）；生命周期/跨子模块真实装配放 DV；HTTP 契约与三交付面交互放 integration；breaking 契约面用 machine 化 contract_checks 补强。
- 最低层级原则：重复创建/锁定拒绝/引用集等先在 service 层断言（unit），HTTP 层只复证状态映射与下载选择（integration），避免高层重复验证分支逻辑。

## Unit Tests

| function_or_unit | branch_or_condition | covered_behavior | test_file | status | gap_manual_reason |
|------------------|---------------------|-----------------|-----------|--------|-------------------|
| VersionService::create_version | 空版本/成功/重复 409 | 显式创建返回空 apps；重复创建 Conflict | server/tests/unit/versions.rs | covered | - |
| VersionService::publish_app | 版本缺失/锁定/新建/更新 | 404 语义、created 标志、sha256/size/updated_at 刷新 | server/tests/unit/versions.rs | covered | - |
| VersionService::delete_app | 版本缺失/锁定/app 缺失/成功 | 删除后引用集移除；404/409 | server/tests/unit/versions.rs | covered | - |
| 版本/app 输入校验 | 空 version、空 app、非法字符、锁定不存在版本 | 空/非法输入 InvalidInput、锁定缺失 NotFound | server/tests/unit/versions.rs | covered | - |
| VersionService::lock | 权限/不存在/幂等 | owner 可锁、成员拒绝、重复锁定 200 | server/tests/unit/versions.rs | covered | - |
| VersionService::list/get | 空版本/app 聚合/latest 倒序 | 单版本返回全部 app；latest 取最近创建 | server/tests/unit/versions.rs | covered | - |
| VersionService::referenced_file_ids | 更新/删除后引用变化 | 旧文件离开 keep 集合 | server/tests/unit/versions.rs | covered | - |
| 权限拒绝 | 只读成员/只读 token | 建版本/发布/锁定拒绝 | server/tests/unit/versions.rs | covered | - |
| admin-web client 新方法 | createVersion/uploadApp/deleteApp/lockVersion | 请求形状、bearer、409 映射、204 处理 | admin-web/tests/unit/client.test.ts | covered | - |
| CLI 命令编排 | publish/download/versions 改造 | 命令参数与输出（app 缺省、锁定状态、app 列表） | - | manual | 无独立纯函数；在 DV/集成 mock 层级以真实命令流覆盖（dv-cli/integration-cli） |
| ProjectDetailPage 交互 | 创建版本/上传/锁定/删除 | 页面交互行为 | - | manual | 以 build + integration stub 覆盖契约调用；未新增组件级渲染断言，UI 情境属残余风险 |

## DV Tests

| workflow | kind | entry | expected_result | test_file_or_script | status | gap_manual_reason |
|----------|------|-------|-----------------|---------------------|--------|-------------------|
| server 版本生命周期（创建/409/多 app/更新/锁定/写拒绝/删除/读取） | lifecycle | dv_full_workflow_with_tokens_and_gc | 全流程断言通过 | server/tests/dv_tests.rs | covered | - |
| server 主流程（权限协作、token、匿名、gc） | main | dv_full_workflow_with_tokens_and_gc | 协作者/权限/回收断言通过 | server/tests/dv_tests.rs | covered | - |
| server 失败流（未授权/重复创建/锁后写） | failure | dv_full_workflow_with_tokens_and_gc | 拒绝语义成立 | server/tests/dv_tests.rs | covered | - |
| server 配置变体（归档上限） | config | dv_full_workflow_with_tokens_and_gc | 超限 ingest 失败 | server/tests/dv_tests.rs | covered | - |
| server 持久化（关库重开） | persistence | dv_persistence_across_reopen | 项目/版本可再次读取 | server/tests/dv_tests.rs | covered | - |
| cli 主流程（登录→发布→下载→查询） | main | dv_full_publish_download_versions_workflow | 0 退出、内容与 sha 校验 | cli/tests/dv_tests.rs | covered | - |
| cli 失败流（未登录/409/锁后删除） | failure | dv_no_credential_and_409_failure_workflows | 退出码 2/4 | cli/tests/dv_tests.rs | covered | - |
| cli 凭据持久化/登出重登 | persistence | dv_full_publish_download_versions_workflow | 凭据可重开、登出清空 | cli/tests/dv_tests.rs | covered | - |
| web 构建产物 | main | test:dv（build + verify-dist） | dist 非空且含入口 | admin-web/tests/dv/verify-dist.mjs | covered | - |

## Integration Tests

| contract_or_flow | modules_involved | success_case | failure_case | test_file | status | gap_manual_reason |
|------------------|------------------|--------------|--------------|-----------|--------|-------------------|
| 版本显式创建/重复 | http↔versions 服务 | POST 201 apps=[] | 重复 POST 409 | server/tests/api_integration.rs | covered | - |
| app 发布/更新/全量查询 | http↔files↔versions | PUT 201/200；GET 返回 2 个 app | 锁定后 PUT/DELETE 409 | server/tests/api_integration.rs | covered | - |
| 按 app 下载/缺省语义 | http↔files | 指定 app 200 且 sha 一致 | 多 app 缺省 422 | server/tests/api_integration.rs | covered | - |
| 输入边界与下载缺省/404 | http↔versions | 单 app 无 ?app 200、显式 app 200 | 空版本 422、非法 app 名 422、错误 app 404 | server/tests/api_integration.rs | covered | - |
| token/协作者权限 | tokens↔permissions↔versions | 只读可读 | 只读创建 403 | server/tests/api_integration.rs | covered | - |
| CLI↔v1 mock 新命令面 | cli↔apiclient | new-version/lock/delete-app 成功 | 重复创建 409、锁后删除 409、缺失 404 | cli/tests/api_integration.rs | covered | - |
| web↔stub 下载与生命周期 | web client↔API stub | 带 app 下载成功 | 匿名 private 401 | admin-web/tests/integration/contract.test.ts | covered | - |
| breaking 契约闭包 | consumer-closure-check + negative fixture | 新路径编译/旧符号扫描通过 | 旧 publish 编译失败 | testing/negative-old-publish.sh | covered | - |

## Definition of Done

- `testplan.yaml` 全部启用步骤（contract + unit + dv + integration）经统一入口 `<module>/<task-name> all` 一次运行通过，run artifact 记录各步骤退出码与 change_ids。
- 每个 change_id 在 Direct Change Coverage 中有 mapping；七类 case type 与六类 design element 全部出现且状态合法。
- 所有测试文件经同一入口可达，无 ad hoc 命令。
