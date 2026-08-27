# 上传 multipart 必须包含非空 file part

- Status: complete
- Owner module: filehub（filehub-server）
- Task manifest: docs/versions/v0.1/modules/filehub/036-require-upload-file-part/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/036-require-upload-file-part/proposal.md
- Affected paths: `server/src/versions/upload.rs`；`server/tests/unit/upload.rs`；`server/tests/api_integration.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `MultipartParser::finish()` 原状只校验是否到达结束边界，`seen_file` 未被使用。
  本次在完成校验中同时要求：必须遇到结束边界、必须出现过 `file` part、且
  `file_bytes > 0`。缺失 `file` part 返回
  `multipart body missing required file part`，显式 0 字节 `file` part 返回
  `multipart file part is empty`。
- HTTP 处理器不新增代码：新增的解析错误沿既有 `upload_error` 失败路径返回
  422，并复用 031 已保证的“文件已入库但请求失败时 discard”清理语义。
- 测试选择 parser 层做单元回归（任意分块喂入都必须在 finish 失败），并在
  真实 HTTP 层做“只有 sha256、没有 file”与“显式空 file part”的集成回归，
  断言 422、版本 apps 为空、files 表与归档目录均无残留。修复前该回归实测
  失败（仅 sha256 的请求返回 201 并发布 0 字节 app），修复后通过，形成
  red-to-green 证据。

## Risk Screen

- Public contract, protocol, or CLI change: no（契约文档本就声明 file 必填，
  本次是把既有契约落实到校验层）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（不改认证/授权；属于上传
  输入校验缺口修复，消除通过空流哈希发布 0 字节产物的契约违反路径）
- Concurrency, lifecycle, or runtime integration change: no（校验点仍在既有
  finish 失败路径内，不改变流式收流与 ingest 时序）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server` 全量（unit_tests 43/43、
  api_integration 5/5、dv_tests 2/2）；定向回归
  `parser_rejects_missing_file_part`、`parser_rejects_empty_file_part` 与
  `upload_rejects_missing_or_empty_file_part` 均通过；修复前同一组回归为
  red（只提交 sha256 时 HTTP 返回 201 并可发布空 artifact）
- Result: pass
- Residual risk or follow-up: 一次整包并行运行时 `api_login_session_and_token_flow`
  出现偶发 502（登录路径与本次改动无关，隔离与后续连续整包运行均绿灯），已记入
  完成报告作为环境性观测；如需根治可在测试基建单独立项。
