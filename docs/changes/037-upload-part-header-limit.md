# 修复 multipart part 头超限误判（大网络 chunk 混入内容）

- Status: complete
- Owner module: filehub（filehub-server versions/upload 解析器）
- Task manifest: `docs/versions/v0.1/modules/filehub/037-upload-part-header-limit/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/037-upload-part-header-limit/proposal.md`
- Affected paths: `server/src/versions/upload.rs`、`server/tests/unit/upload.rs`、
  `docs/changes/037-upload-part-header-limit.md`、
  `docs/versions/v0.1/modules/filehub/037-upload-part-header-limit/completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷：`Phase::Headers` 原先先对 `pending.len()` 与 `max_header_bytes`（8 KiB）
  比较，之后才搜索 `\r\n\r\n`；单个网络 chunk 同时包含合法 part 头与超过 8 KiB
  的后续内容（file/字段数据）时，合法上传被误报
  `multipart part headers exceed limit`（HTTP 层 422）。
- 修复：调换判定顺序——先 `find_subslice(&self.pending, b"\r\n\r\n")`；
  找到时只按头部字节数 `end` 与 `max_header_bytes` 比较；未找到时才用累计
  `pending.len()` 封顶并返回 `Blocked`，保留对真实超限头部的尽早拒绝。
- 测试：`server/tests/unit/upload.rs` 新增 3 项回归——(1) 单 chunk 内小 part 头 +
  16 KiB 文件内容不再误报且内容原样还原；(2) 大内容 body 在任意切分
  （1..=len+5）下解析一致；(3) 真正超过 8 KiB 的 part 头仍返回原错误文案。

## Risk Screen

- Public contract, protocol, or CLI change: no（HTTP 请求/响应形状不变，仅修
  复此前误拒的合法请求判定）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（真实超限头与未终止头部累积
  仍受 8 KiB 上限约束，无 DoS 面变化）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests`（43/43 通过，
  含新增 3 项 header 切分/上限用例与 036 在制 2 项 parser 单测）；
  `--test api_integration`（除 036 在制失败用例外 4/4 通过，
  `upload_security_boundaries` 覆盖真实 HTTP 上传限制回归）；
  `--test dv_tests`（2/2 通过）
- Result: pass
- Residual risk or follow-up: 工作树内 036 在制内容中
  `upload_rejects_missing_or_empty_file_part` 集成用例当前失败（拒绝后 files
  表残留 1 条记录），属 036 finish/discard 路径未完成内容，与本任务 Headers
  改动无关（任务开始即存在、失败点不经过本次修改分支），由 036 收尾。
