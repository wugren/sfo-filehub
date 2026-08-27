---
task_manifest: task.yaml
status: draft
---

# 上传鉴权前置与流式收流 Testing

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| none | 本任务不拆分 `testing/` 子文档；测试集中在 `server/tests/unit/`、`server/tests/dv_tests.rs` 与 `server/tests/api_integration.rs` | 上传鉴权前置、流式 multipart 解析、流式入库、解压上限与回归 |

## Unified Test Entry

- Machine-readable task plan: `docs/versions/v0.1/modules/filehub/026-upload-authz-body-bounds/testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/026-upload-authz-body-bounds all`
- Single-task boundary：只运行本任务 testplan 注册的 contract/unit/dv/integration 步骤，不运行模块级、`all all` 或根快捷入口。
- Registration：新增断言全部在 `server/tests/unit/`（`unit_tests` 目标）、`server/tests/api_integration.rs` 中注册，经 testplan.yaml 可达。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| http（authz） | 上传路由授权前置 | none | PUT 上传在读取 body 前完成 artifacts:write 判定；匿名 401、越权 403；业务层纵深校验保留 | 匿名超大脏 body 仍 401（未进入解析） | integration/unit | server/tests/api_integration.rs | covered | not-applicable |
| versions（upload.rs） | 增量 multipart 解析与编排 | none | boundary 跨任意 chunk 切分正确；file part 直出、sha256 小缓冲；实时大小/字段/头部/总量上限 | 收尾 `--`/CRLF 切分、单 part 与双 part、超大 file part、收尾后尾随数据 | unit | server/tests/unit/upload.rs | covered | not-applicable |
| storage | 流式入库与有界解压 | none | 流式写临时文件实时计数；超 max_archive_bytes 即清理拒绝；解压超 max_decompressed_bytes 拒绝；sha256/size 一致 | 超限、压缩炸弹、duplex 分块写入、sha 不匹配、GC 清理 | unit/integration | server/tests/unit/upload_ingest.rs、storage.rs、api_integration.rs | covered | not-applicable |
| model | FilesConfig 可选解压上限 | none | 缺省推导 max_archive_bytes×20；显式配置生效 | 小上限配置驱动集成用例 | integration/dv | server/tests/api_integration.rs、dv_tests.rs | covered | not-applicable |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| multipart 解析正确性 | 任意 chunk 切分、单/双 part、二进制保留 | `cargo test --test unit_tests` | 所有切分下 file 字节与 sha 字段还原一致 | unit | server/tests/unit/upload.rs | covered | not-applicable |
| 流式入库链路 | duplex 分块写入 → spool → 校验 → 落位 | `cargo test --test unit_tests` | record.size/sha256 与归档一致；失败清理 | unit | server/tests/unit/upload_ingest.rs | covered | not-applicable |
| 上传-发布-下载主流程 | 合法上传、重复发布更新、下载内容一致 | `cargo test --test api_integration` | 201/200 与下载字节一致 | integration | server/tests/api_integration.rs | covered | not-applicable |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| v1 PUT 上传 | 授权前置 + 流式收流 + 422 上限语义 | 合法上传 201/200 | 匿名 401；归档超限 422；解压超限 422 | integration | server/tests/api_integration.rs | covered | not-applicable |
| sfo-http 0.8 + sfo-account shim 构建面 | 依赖升级不影响编译 | workspace 全目标编译 | 无（编译即断言） | contract | testplan.yaml `contract_repository_compile_closure` | covered | not-applicable |
| docs/api/v1-contract.md | 上传流程与上限契约说明 | 文档描述与实现一致 | 无 | manual | docs/api/v1-contract.md | covered | 契约文档由实现同步更新，无独立自动化断言 |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-upload-authz-gate | design/http.md、design/versions.md；实现 server/src/http/authz.rs、versions/http.rs | VAL-upload-authz-gate | integration | upload-integration | no | |
| fh-http-body-limit | design/versions.md；实现 server/src/versions/upload.rs、versions/http.rs、server/Cargo.toml | VAL-upload-stream-limits | unit | upload-unit | no | |
| fh-upload-size-and-decompression-bounds | design/storage.md、design/model.md；实现 server/src/storage/、model/config.rs | VAL-decompression-bounds | unit | upload-unit | no | |
| fh-upload-security-tests | design.md、design/versions.md；实现 server/tests/unit/、api_integration.rs | VAL-upload-regression | dv | upload-dv | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-upload-authz-gate | normal | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-upload-authz-gate | boundary | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-upload-authz-gate | negative | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-upload-authz-gate | error | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-upload-authz-gate | compatibility | no | VAL-upload-regression | integration | not-applicable | 401/403 映射保持既有契约，无兼容回退需求 |
| fh-upload-authz-gate | lifecycle | yes | VAL-upload-regression | integration | covered | not-applicable |
| fh-upload-authz-gate | cross-module | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-http-body-limit | normal | yes | VAL-upload-stream-limits | unit | covered | not-applicable |
| fh-http-body-limit | boundary | yes | VAL-upload-stream-limits | unit | covered | not-applicable |
| fh-http-body-limit | negative | yes | VAL-upload-stream-limits | unit | covered | not-applicable |
| fh-http-body-limit | error | yes | VAL-upload-stream-limits | unit | covered | not-applicable |
| fh-http-body-limit | compatibility | no | VAL-upload-regression | unit | not-applicable | multipart 客户端契约不变，请求/响应形状无差异 |
| fh-http-body-limit | lifecycle | yes | VAL-upload-regression | integration | covered | not-applicable |
| fh-http-body-limit | cross-module | yes | VAL-upload-regression | unit | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | normal | yes | VAL-decompression-bounds | unit | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | boundary | yes | VAL-decompression-bounds | unit | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | negative | yes | VAL-decompression-bounds | unit | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | error | yes | VAL-decompression-bounds | unit | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | compatibility | no | VAL-upload-regression | unit | not-applicable | max_decompressed_bytes 为可选配置项，旧配置缺省行为被默认值替代 |
| fh-upload-size-and-decompression-bounds | lifecycle | yes | VAL-upload-regression | integration | covered | not-applicable |
| fh-upload-size-and-decompression-bounds | cross-module | yes | VAL-decompression-bounds | integration | covered | not-applicable |
| fh-upload-security-tests | normal | yes | VAL-upload-regression | integration | covered | not-applicable |
| fh-upload-security-tests | boundary | yes | VAL-upload-stream-limits | unit | covered | not-applicable |
| fh-upload-security-tests | negative | yes | VAL-upload-authz-gate | integration | covered | not-applicable |
| fh-upload-security-tests | error | yes | VAL-decompression-bounds | unit | covered | not-applicable |
| fh-upload-security-tests | compatibility | no | VAL-upload-regression | unit | not-applicable | 测试注册不引入对外行为，无兼容断言 |
| fh-upload-security-tests | lifecycle | yes | VAL-upload-regression | integration | covered | not-applicable |
| fh-upload-security-tests | cross-module | yes | VAL-upload-regression | integration | covered | not-applicable |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | design/model.md（FilesConfig 可选上限）、design/versions.md（UploadLimits） | 缺省 20× 推导、显式小上限、max_field/header 上限、总请求体上限 | unit | covered | not-applicable |
| state-transition | design/versions.md（MultipartParser 相位状态机） | 任意 chunk 切分下 Preamble/Headers/Content/Boundary/Closing/Finished | unit | covered | not-applicable |
| failure-path | design/storage.md（spool 失败清理）与 design.md（上传失败不落库） | 归档超限、解压超限、校验失败均返回 422 且无文件残留（gc 断言） | unit | covered | not-applicable |
| error-handling | design/http.md（authz 401/403）与 422 上限语义 | 匿名 401、越权 403、超限 422 | integration | covered | not-applicable |
| invariant | 001 risk-profile：权限校验 -> 文件入库 -> 版本落库 | 匿名上传 401 先于 body 消费；契约文档同步 | integration | covered | not-applicable |
| concurrency | design.md（duplex 背压 + 并行 ingest） | 分块写入与入库并行、多 part 顺序一致 | unit | covered | not-applicable |

## Validation Rationale

- 授权前置的风险点（未认证内存/CPU DoS）用「匿名 + 1 MiB 脏 body 仍 401」的集成断言证明鉴权先于 body 消费。
- 请求体无界风险用「multipart 解析器任意 chunk 切分还原一致」与「duplex 分块写入 = 入库成功」在单元层覆盖，配合真实 HTTP 流式链路集成回归。
- 内存放大用流式临时文件 + 实时计数（O(1)）实现；超限拒绝在写入中段发生，集成用例用「小上限配置 + 超限归档/压缩炸弹」断言 422 与不落库。
- 压缩炸弹用「小压缩包、大解压量」的 validate/ingest 单元断言与集成断言双重覆盖。
- 回归范围：单元（30+ 项）、DV（生命周期/主流程/失败/配置/持久化）、集成（登录/项目/版本/token 及上传安全边界）全量执行。

## Unit Tests

| function_or_unit | branch_or_condition | covered_behavior | test_file | status | gap_manual_reason |
|------------------|---------------------|-----------------|-----------|--------|-------------------|
| MultipartParser::feed/finish | 任意 chunk 切分（1..=len+5） | 双 part（file+sha256）还原一致 | server/tests/unit/upload.rs | covered | not-applicable |
| MultipartParser::feed/finish | 单 file part（reqwest 式） | 文件字节与闭包边界一致 | server/tests/unit/upload.rs | covered | not-applicable |
| MultipartParser::feed | 二进制 gzip 内容 | 任意切分下字节零改动 | server/tests/unit/upload.rs | covered | not-applicable |
| MultipartParser 上限 | file 超 max_archive_bytes、字段超限、收尾尾随 | 422/错误文案 | server/tests/unit/upload.rs | covered | not-applicable |
| validate_targz | 解压总量上限 | 压缩炸弹拒绝、宽松上限放行 | server/tests/unit/upload_ingest.rs | covered | not-applicable |
| UploadStream::from_reader + ingest | duplex 任意分块写入 | 入库 size/sha 与源一致 | server/tests/unit/upload_ingest.rs | covered | not-applicable |
| UploadStream::from_bytes + ingest | sha 期望匹配/不匹配 | 匹配落库、不匹配拒绝 | server/tests/unit/storage.rs | covered | not-applicable |
| SqliteFileStore::ingest | max_archive_bytes 写入期超限 | 超限即拒绝且清理 | server/tests/unit/storage.rs、dv_tests.rs | covered | not-applicable |
| 既有 ingest 迁移 | UploadStream 新类型 | versions/storage/dv 全部调用点编译且行为不变 | server/tests/unit/{storage,versions}.rs、dv_tests.rs | covered | not-applicable |

## DV Tests

| workflow | kind | entry | expected_result | test_file_or_script | status | gap_manual_reason |
|----------|------|-------|-----------------|---------------------|--------|-------------------|
| 上传-发布-更新-锁定-删除 | lifecycle | `cargo test --test dv_tests` | 全生命周期正常 | server/tests/dv_tests.rs | covered | not-applicable |
| 登录-上传-下载主流程 | main | `cargo test --test dv_tests` | 版本与文件回流一致 | server/tests/dv_tests.rs | covered | not-applicable |
| 超限归档拒绝 | failure | `cargo test --test dv_tests` | 422 且无落库 | server/tests/dv_tests.rs | covered | not-applicable |
| max_decompressed_bytes 配置变体 | config | `cargo test --test dv_tests` | 小上限生效、缺省推导 | server/tests/dv_tests.rs | covered | not-applicable |
| files 表与 GC 持久化 | persistence | `cargo test --test dv_tests` | 记录一致性、孤儿清理 | server/tests/dv_tests.rs | covered | not-applicable |

## Integration Tests

| contract_or_flow | modules_involved | success_case | failure_case | test_file | status | gap_manual_reason |
|------------------|------------------|--------------|--------------|-----------|--------|-------------------|
| 匿名上传授权前置 | http/authz、versions、storage | 合法上传 201/200 | 匿名超大脏 body 401 | server/tests/api_integration.rs | covered | not-applicable |
| 归档大小上限 | versions、storage、http | 合法小归档 201 | 小配置 + 超限归档 422 | server/tests/api_integration.rs | covered | not-applicable |
| 解压上限（压缩炸弹） | versions、storage | 宽松上限正常 | 小压缩包大解压量 422 | server/tests/api_integration.rs | covered | not-applicable |
| 登录/项目/版本主流程回归 | account、permissions、versions、storage | 既有全流程绿 | 越权/锁定冲突 403/409 | server/tests/api_integration.rs | covered | not-applicable |
| 上传-发布-下载闭环 | versions、storage、http | 下载字节与上传归档一致 | 错误 app/缺失 app 404/422 | server/tests/api_integration.rs | covered | not-applicable |

## Definition of Done

- 四类单元/DV/集成测试全部执行通过（`cargo test -p filehub-server` 与任务级 `test-run.py filehub/026-upload-authz-body-bounds all`）。
- 匿名 401、归档超限 422、压缩炸弹 422 与合法上传回归均有对应断言。
- workspace 全目标编译通过（contract 步骤）。
- 所有 `change_id` 在 Direct Change Coverage/Case-Type Coverage 中有闭环记录，无未说明 gap。
