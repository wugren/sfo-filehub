---
task_manifest: task.yaml
status: approved
---

# 修复 multipart part 头超限误判（大网络 chunk 混入内容）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 回复「确认」，确认采纳提案（先查找
  `\r\n\r\n`，找到时只比较 `end`，未找到时才比较累计缓冲长度）并接受建议的
  standard 层级。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本任务是 filehub-server 单一生产模块（versions/upload 解析器）的上传校验行为
    修复，并需要新增解析器回归测试验证“头部未超限但 pending 混入后续内容”不再
    误报；
  - 不满足 trivial：修改的是上传校验行为（此前被误拒绝的合法大块上传修复后
    变为可成功解析），属于运行时行为变更且需要确定性回归验证，与 036 同类
    任务沿用同一 local 判定；
  - 未触发 high-risk：只在 `MultipartParser::Phase::Headers` 内部调整超限检查
    的判定顺序与口径，不新增/修改公开 API 契约、不涉及数据 schema/迁移、不新增
    依赖、不改变认证授权或存储边界，也不触碰发布/兼容/回滚语义；
  - 按 standard 的默认 bounded bugfix 流程执行：pre-edit 基线 -> 实现 -> 验证 ->
    变更记录 `docs/changes/037-upload-part-header-limit.md` -> completion-report。
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，按本提案
  推荐范围（仅调整 Headers 阶段检查顺序并补回归测试）与 standard 层级执行。

## Background and Goal

- 现象（中危缺陷）：`server/src/versions/upload.rs` `Phase::Headers`（第 121 行起）
  先对 `self.pending.len()` 与 `max_header_bytes`（8 KiB）比较，之后才搜索
  `\r\n\r\n`；`pending` 在 `feed` 中先追加整个网络 chunk。
- 因此：单个 body chunk 同时包含正常 multipart part 头和超过 8 KiB 的后续内容
  （file 或字段数据）时，整个缓冲区被当成头部计数，合法上传被误报
  `multipart part headers exceed limit`（HTTP 层 422）。现有测试数据较小，
  未覆盖该切分形态。
- 目标：先查找 part 头结束位置 `\r\n\r\n`；找到时只比较头部字节数（`end`）
  与 8 KiB 上限；未找到时才用累计 `pending.len()` 作为“头部仍在累积中”的比较
  口径，保留对真正超限头部的尽早拒绝。

## Scope

### In scope

- `server/src/versions/upload.rs`：调整 `Phase::Headers` 的检查顺序——先
  `find_subslice(&self.pending, b"\r\n\r\n")`；找到后按 `end` 比较
  `max_header_bytes`，未找到时按 `pending.len()` 比较后返回 `Blocked`；
- `server/tests/unit/upload.rs`：新增回归用例——
  (1) 单个 chunk 同时包含小 part 头与超过 8 KiB 的 file 内容，`feed` 必须成功
  产出事件且不报 headers 超限；
  (2) 任意切分（百叶窗 1..=len+5）下双 part 混合大内容仍能还原一致；
  (3) 真正超过 8 KiB 的 part 头仍返回 `multipart part headers exceed limit`；
- 按 standard 层级产出 `docs/changes/037-upload-part-header-limit.md` 与任务包
  `completion-report.md`（中文正文）。

### Out of scope

- 不切换 multer / actix-multipart 等第三方 multipart 解析库；
- 不修改外层 HTTP 头解析（仍由 actix-web/sfo-http 负责）与 8 KiB 上限值；
- 不修改错误文案、HTTP handler、`docs/api/v1-contract.md` 或客户端；
- 不触碰 036 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 修复完全位于解析器内部，`versions/http.rs` 复用现有 `upload_error` ->
  `ApiError::invalid_input`（422）失败路径，存储 ingest 与 discard 语义不变；
- 负向超限行为保持不变，仅消除“头部未超限但被 pending 总量误判”的假阳性。

## Requirement Review

- 需求合理：`max_header_bytes` 的语义是“单个 part 头部上限”，不应把尚未解析的
  内容字节计入；先定位头部结束位置再比较是更正确的判定顺序，且不削弱对真实
  超限头部的拒绝。
- 方向选择：采纳 review 建议的判定顺序（先找 `\r\n\r\n`，找到比较 `end`，未找到
  比较累计长度），改动小、只影响 Headers 分支，风险可控。
- 材料风险/权衡：修改后单个 chunk 混入超过 8 KiB 内容时不再误拒，但该内容随后
  仍在 Content 阶段受 `max_archive_bytes` / `max_field_bytes` 实时计数约束，
  不产生新的未受控缓冲；`pending` 只保留到边界/分隔符所需长度。
- 待确认问题：无（推荐与 review 建议一致，即“找到时只比较 `end`”）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-upload-header-limit-order | `Phase::Headers` 先搜索 `\r\n\r\n`：找到时按 `end` 比较 `max_header_bytes`，未找到时按 `pending.len()` 比较 | 仅 `server/src/versions/upload.rs` Headers 分支 | 超限头部仍被同一错误拒绝；0 字节语义不变 | 混入 >8KiB 内容的单 chunk 解析成功；真实超限头单测仍报错 | 不调整上限值/文案，不切换第三方解析库 |
| P-002 | fh-upload-header-limit-regression-tests | 新增回归单测：小头+大内容单 chunk、任意切分还原、真超限头负向 | `server/tests/unit/upload.rs` | 用例只覆盖解析器，不新增 HTTP 集成面 | 新增用例通过且既有上传用例回归全绿 | 不新增 HTTP-level 测试（行为面未变） |

## Success Criteria

- 单个 body chunk 含合法 part 头 + 超过 8 KiB 的后续内容时，`feed` 不再返回
  `multipart part headers exceed limit`，file 内容原样产出并累计；
- 真正超过 8 KiB 的 part 头仍返回同一错误信息；
- `cargo test -p filehub-server` 全量通过（含既有上传/入库用例）；
- 按 standard 流程产出并校验 `docs/changes/037-upload-part-header-limit.md` 与
  `completion-report.md`。

## Risks

- 行为放宽范围：仅“头部本身未超限但 pending 总量超过 8 KiB”的合法大块请求
  从 422 变为成功；真实超限头与超长头部累积仍被 422 拒绝，无 DoS 面变化；
- 工作树存在大量未提交的用户改动（025-036 等在制内容），本任务只修改提案
  列出的文件，不运行仓库级格式化，不触碰无关在制改动。
