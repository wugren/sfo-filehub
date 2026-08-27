# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/031-discard-orphan-upload.md

## Delivery Summary
- Outcome:
  - `server/src/versions/http.rs` 上传 handler：当 `upload_error` 已设置且
    `ingest_task` 结果为 `Ok(file)` 时，先 `files.discard(&file.file_id)`
    再返回 422；畸形 multipart 分帧、trailer/结束边界失败或中途断流不再
    残留孤儿文件与 `files` DB 记录；
  - `server/tests/api_integration.rs` 新增回归测试：流式分帧发送完整 file
    part 后在第二帧触发空字段解析失败，断言 422 且重新打开 SQLite 后
    `files` 计数为 0（修复前 red：残留 1 条；修复后 green）；
  - `server/Cargo.toml` dev-dependencies 新增 `futures-util` 并开启 reqwest
    `stream` feature（仅测试构建），Cargo.lock 同步。
- Handoff: `cargo test -p filehub-server` 全量通过（4 api_integration +
  2 dv + 33 unit）；改动仅限上述预定文件；工作区其它未提交在制内容（026 等
  任务）未触碰，pre-edit 基线确保变更清单只含本任务增量。

## Proposal Consistency
| Change ID | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-upload-orphan-discard | 解析失败且文件已入库时先 discard 再 422；不改解析器/存储层/成功路径 | proposal.md P-001 + In scope | `http.rs` 422 早退分支新增 `joined` Ok 时 discard；回归测试 red->green；sha256 不匹配/publish 失败分支差异为空 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `http.rs` 上传 handler 全链路：feed 循环、upload_error 分支、joined/ingest panic、sha256 不匹配与 publish 失败 discard 分支 | 逐分支推演：file 已入库后 frame 错误、trailer 帧、feed/finish 错误、ingest 失败、二者同时失败、成功路径 | 只有 upload_error+Ok(file) 分支缺 discard，修复后各路径均收敛；错误响应顺序与 500/422 语义不变 | pass |
| boundaries-and-failure-paths | multipart 分帧边界的 red->green 实测、`feed` 同帧报错丢弃事件行为、`drop(writer)` 后 ingest join 时序、SQLite 重开计数断言 | 尝试同帧合并与跨帧停顿两种送达方式，确认只有跨帧停顿真正复现孤儿；检查合并且 joined=Err 时无残留、discard 失败仍按 422 返回 | 同帧合并时解析错误会吞掉文件事件、ingest 失败且无孤儿，属既有解析语义非本缺陷；修复后跨帧路径清理成功 | pass |
| regression-and-side-effects | git diff 与 baseline diff、全量 `cargo test -p filehub-server`、dev 依赖变更对生产构建的影响、026 等在制未提交文件 | 核对 http.rs 的 026 流式上传改动未被动；api_integration 全部 4 用例与 unit/dv 用例无回归；Cargo.lock 包集合无新增 | 4 api_integration + 2 dv + 33 unit 全绿；变更清单经 baseline 对比仅含本任务 4 个文件；无生产依赖变化 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server --test api_integration
  upload_parse_failure_after_ingest_discards_orphan` red->green；
  `cargo test -p filehub-server` 全量 39 项通过
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `ingest_task.await` 返回 `Err(JoinError)` 时无 file_id 可用 | ingest 任务 panic 仍可能遗留已写入文件，但本 handler 无从取得 file_id 清理；保持现有 500 语义，建议后续在 ingest 内部自清理或任务级 GC 跟进 | no |
| F-2 | low | 集成测试依赖两帧之间 200ms 停顿保证跨帧送达 | 若未来 HTTP 层整包缓冲/合并分帧，该用例可能退化为不触发孤儿分支（仍断言无残留、通过但不侦测缺陷）；已记入 change record 后续项 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001 的最小修复与回归测试均已落地并经 red->green 与全量测试
  验证；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，未发现阻塞性
  缺陷；F-1/F-2 为非阻塞后续项。
