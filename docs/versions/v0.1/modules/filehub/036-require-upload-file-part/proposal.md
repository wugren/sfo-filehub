---
task_manifest: task.yaml
status: approved
---

# 上传 multipart 缺少 file part（或空 file）时禁止发布

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认」，确认采纳提案（含“0 字节空 file 一并拒绝”
  的推荐范围）并接受建议的 standard 层级。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本任务是 filehub-server 单一生产模块（versions/upload 解析 -> HTTP 处理器 ->
    storage 落库）的运行时行为修复，并需要新增 HTTP 回归测试验证“只有 sha256、
    没有 file”的 multipart 请求被 422 拒绝且不发布；
  - 不满足 trivial：修改的是上传校验行为（此前可成功发布的请求将变为 422），
    属于运行时行为变更且需要回归验证，不是文档/配置性质的单一局部改动；
  - 未触发 high-risk：修复只是把 `docs/api/v1-contract.md` 已声明的“file 为必填”
    落实到解析校验层，不新增/修改公开 API 契约、不涉及数据 schema/迁移、不新增
    依赖、不改变发布/兼容/回滚语义，也不触碰认证授权边界；
  - 按 standard 的默认 bounded bugfix 流程执行：pre-edit 基线 -> 实现 -> 验证 ->
    变更记录 `docs/changes/036-require-upload-file-part.md` -> completion-report。
- Proposal and tier confirmation: 等待用户确认；用户只回复「确认」时按本提案
  推荐范围（缺 file part 与 0 字节空 file 均拒绝）与 standard 层级执行。

## Background and Goal

- 现象（中危缺陷）：`server/src/versions/upload.rs` 的 `MultipartParser::finish()`
  只校验是否到达结束边界（`Phase::Finished`），`seen_file` 虽然被记录但未参与
  完成校验；`server/src/versions/http.rs` 上传处理器只强制校验 `sha256` 字段与
  哈希匹配；`server/src/storage/store.rs` 的 `ingest` 接受零字节流并落库。
- 因此：只提交 `sha256`（取空内容的 SHA-256，即
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`）而不带
  `file` part 的 multipart 请求，可以发布一个零字节 app artifact，违反
  `docs/api/v1-contract.md` 第 48 行声明的“file(.tar.gz) 必填”。
- 用户结论：不支持发布空文件。目标是把“file part 必须存在”落实到解析完成校验，
  并补充“只有 sha256、没有 file”的 HTTP 回归测试（空 file part 是否一并拒绝见
  Requirement Review 待确认项，推荐一并拒绝）。

## Scope

### In scope

- `server/src/versions/upload.rs`：`finish()` 增加 `seen_file` 校验——缺少 `file`
  part 时返回解析错误（HTTP 侧沿用现有失败路径返回 422 并 discard 已入库临时文件）；
- 按用户“不支持发布空文件”的结论，`finish()` 同时要求 `file_bytes > 0`，显式提交
  0 字节 `file` part 同样拒绝（推荐范围，待确认）；
- 单元测试：`server/tests/unit/upload.rs` 增加“无 file part”“空 file part”用例，
  断言 `finish()` 返回错误；
- HTTP 回归测试：`server/tests/api_integration.rs` 增加“只有 sha256、没有 file”
  的 multipart PUT，断言 422、版本 `apps` 为空且不残留孤儿文件记录；
- 按 standard 层级产出 `docs/changes/036-require-upload-file-part.md` 与任务包
  `completion-report.md`（均使用中文描述正文）。

### Out of scope

- 不修改 `sha256` 必填/校验语义（缺失与不匹配仍按现有 422 处理）；
- 不修改 `max_archive_bytes` 与 ingest 对非空流的接受逻辑；
- 不做解压/`tar.gz` 内容校验；0 字节以外的大小仍由现有 `max_archive_bytes` 限制；
- 不改 `docs/api/v1-contract.md`（契约已声明 `file` 必填，本任务只是落实契约）；
- 不改 CLI/admin-web/前端；不改认证授权路径；
- 不触碰除上述文件外的既有未提交用户改动。

### Boundary with neighboring modules

- 校验点放在解析器 `finish()`（parser 负责 multipart 结构合法性，`seen_file`/
  `file_bytes` 本就是其解析状态），HTTP 处理器复用现有 `upload_error` 失败路径
  （含 031 已修复的“文件已入库后解析失败仍 discard”语义）；
- storage 层不新增“拒绝 0 字节”校验，零字节判定只由 parser 完成，避免在
  `ingest` 通用接口上放大改动面。

## Requirement Review

- 需求合理：契约声明 `file` 必填但当前校验链路未落实，属于解析完成校验的缺口；
  修复后的 422 行为与文档契约一致，不构成契约破坏。
- 方向选择：在 `finish()` 中同时校验 `seen_file && file_bytes > 0`。这样
  “只有 sha256、没有 file”与“显式 0 字节 file”两类请求都在进入发布前被拒绝；
- 材料风险/权衡：校验发生在流式消费结束后，0 字节文件可能已短暂写入
  ingest 管道并落库，但 HTTP 处理器在 `upload_error` 分支已有 discard 语义
  （031 修复），不会在 `files` 表或磁盘残留；
- 待确认问题：是否连“显式提交空的 `file` part（0 字节）”也一并拒绝？用户结论
  “不支持发布空文件”倾向一并拒绝，本提案推荐一并拒绝；若用户只回复「确认」，
  按推荐范围执行。

## Proposal Items

| proposal_id | change_id | requirement | success_evidence |
|-------------|-----------|-------------|------------------|
| P-001 | fh-upload-require-file-part | `MultipartParser::finish()` 必须校验 `seen_file`；缺少 `file` part 的上传在解析完成阶段报错 | 无 file 的 multipart 在 `finish()` 返回 `Err`；HTTP 回归 422 且版本 `apps` 为空 |
| P-002 | fh-upload-reject-empty-file | 按“不支持发布空文件”的结论，`finish()` 同时拒绝 `file_bytes == 0` 的空 `file` part（待确认范围） | 0 字节 file part 的 `finish()` 返回 `Err`；对应单测通过 |
| P-003 | fh-upload-file-part-tests | 新增解析器单测（缺 file / 空 file）与 HTTP 回归（仅 sha256、无 file → 422 不发布） | `cargo test -p filehub-server` 受影响用例与既有上传用例全量通过 |

## Success Criteria

- 仅提交 `sha256`（空流哈希）、不带 `file` part 的 PUT 请求返回 422，版本记录
  `apps` 保持为空，`files` 表/磁盘不残留孤儿文件；
- （推荐范围）显式 0 字节 `file` part 同样返回 422；
- 正常上传（非空 `file` + 合法且匹配的 `sha256`）继续成功，既有
  `upload_security_boundaries` 等上传用例不受影响；
- `cargo test -p filehub-server` 通过；按 standard 流程产出并校验变更记录与
  completion-report。

## Risks

- 行为收紧：此前“只有 sha256、没有 file”的请求本可成功发布 0 字节 artifact，
  修复后变为 422；这与文档契约一致，且 0 字节 `.tar.gz` 不是有效发布产物，不
  存在合理的兼容需求；
- 现有工作树存在大量未提交的用户改动，本任务只修改提案列出的相关文件，不运行
  仓库级格式化、不动其他未提交改动（沿用仓库既有“最小定向修复”约定）。
