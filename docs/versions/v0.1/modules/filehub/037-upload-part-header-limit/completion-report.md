# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/037-upload-part-header-limit.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `MultipartParser::Phase::Headers` 改为先搜索 part 头结束符 `\r\n\r\n`：
    找到时只按头部字节数 `end` 与 `max_header_bytes` 比较，未找到时才按累计
    `pending.len()` 比较并返回 `Blocked`（`server/src/versions/upload.rs`）；
  - 新增 3 项解析器回归单测：单 chunk 内小 part 头 + 16 KiB 文件内容不再误报、
    大内容 body 任意切分（1..=len+5）还原一致、真实超限 part 头仍返回
    `multipart part headers exceed limit`；
  - 8 KiB 上限值、错误文案、HTTP handler、存储 ingest/discard 语义均未改动。
- Handoff: `cargo test -p filehub-server --test unit_tests` 43/43 通过；
  `--test api_integration` 除 036 在制失败用例外 4/4 通过；
  `--test dv_tests` 2/2 通过；未触碰 025-036 等在制内容（036 并行追加的两项
  parser 单测与本次三项目前共存于同一文件且全部通过）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-upload-header-limit-order | `Phase::Headers` 先搜索 `\r\n\r\n`，找到时按 `end` 比较 `max_header_bytes`，未找到时按 `pending.len()` 比较并 `Blocked` | proposal.md P-001 | `upload.rs` Headers 分支（找到 → `end > max_header_bytes` 才报错；未找到 → 累计 > 上限才报错）；真实超限头错误文案不变 | 匹配 | pass |
| fh-upload-header-limit-regression-tests | 新增小头+大内容单 chunk、任意切分还原、真超限头负向三项单测 | proposal.md P-002 | `server/tests/unit/upload.rs` 新增 3 项；`--test unit_tests` 43/43 通过 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `upload.rs` Headers 分支判定顺序与两个比较口径、Preamble→Headers→Content 相位转换、`find_subslice` 语义 | 反向推演：头部结束符在 pending 内且 `end` 未超限时是否误拒（否）；未找到分隔符时是否仍按累计长度封顶（是）；`\r\n\r\n` 自身是否计入头部长度（否，`end` 为起始索引）；Content 相位对同一 chunk 剩余内容是否继续受 file/field/total 上限（是） | 合法大块不再误判；真实超限与未终止超长头部仍被拒绝；无绕过上限或错误计数路径 | pass |
| boundaries-and-failure-paths | 8 KiB 边界（`end == 8192` 允许、`end == 8200` 拒绝）、错误文案、分块停在头分隔符中间、HTTP 422 失败路径与 `files.discard` | 检查结束符被跨 chunk 切分时按累计长度比较的既有语义是否回退；头部恰好等于上限的 off-by-one；超限头在单 chunk 与任意切分下是否一致拒绝 | 边界口径统一为“头部字节数 ≤ 8192”；负向单测覆盖；无 off-by-one；HTTP/存储路径未改动 | pass |
| regression-and-side-effects | `cargo test -p filehub-server` 三个 test binary、真实 HTTP `upload_security_boundaries`、大内容任意切分 sweep、二进制 tar 保留、与 036 在制单测共存 | 排查本改动是否影响缺 file/空 file（036）语义、ingest 流水与隔离并发测试运行稳定性；确认 036 在制集成失败与 Headers 分支无关 | unit 43/43、dv 2/2、api_integration 除 036 在制项外 4/4；并发运行两个 test binary 的偶发失败在隔离串行复跑后消失，判定为测试运行竞争而非产品缺陷 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server`：unit_tests 43/43 通过；
  api_integration 仅 036 在制 `upload_rejects_missing_or_empty_file_part`
  失败（其余 4/4，含 upload_security_boundaries）；dv_tests 2/2 通过；
  037 改动路径仅 `server/src/versions/upload.rs` 与
  `server/tests/unit/upload.rs` 两个文件内的 Headers 分支与 3 项新增单测。
- Result: pass
- Exception reason: 036 在制未完成集成用例导致全量套件不能整体为绿；该用例在
  任务开始时已存在且失败点为 036 finish/discard 路径，不经过本次修改分支；
  037 相关单元、集成与 DV 覆盖均通过，因此 037 目标验证成立。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 036 在制 `upload_rejects_missing_or_empty_file_part` 集成用例失败（拒绝后 `files` 表残留 1 条记录） | 属于 036 finish/discard 语义未完成内容，任务前已存在，需由 036 收尾 | no |
| F-2 | low | 任务期间 036 并行向 `server/tests/unit/upload.rs` 追加 missing/empty parser 单测 | completion 基线 diff 会把该文件整体计入 037 变更面，交付内容仍可区分（037 为 Headers 分支 + 3 项单测） | no |
| F-3 | low | 并行运行 `api_integration` 与 `dv_tests` 时曾出现偶发单项失败 | 隔离串行复跑全绿（除 036 在制项），判定为测试运行竞争而非产品缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002 全部落地：Headers 判定顺序按“先找结束符、找到只比
  `end`”修复，3 项回归单测全绿，unit 43/43 与 dv 2/2 通过，真实 HTTP 上传
  限制回归通过；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用；
  F-1~F-3 均为与 037 无关或在制共存的低危记录，不阻止收尾。
