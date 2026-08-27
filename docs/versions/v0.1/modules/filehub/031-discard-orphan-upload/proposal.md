task_manifest: task.yaml
status: approved
---

# 上传解析失败清理已入库孤儿文件

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认」，批准 standard 层级提案并按该层级完成实现、验证与收尾。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 这是 filehub-server 的单一生产代码缺陷修复（`server/src/versions/http.rs`
    上传 handler 的失败路径），目标是消除已入库但从未被版本引用的孤儿文件与
    DB 记录；
  - 变更面限定在 handler 的 cleanup 分支，不改变公开 API 契约、不触碰数据库
    schema/迁移、不新增依赖，也不改变发布/兼容语义；
  - 但修改的是文件落库后的失败清理语义（资源生命周期/失败路径），需要一条
    回归测试证明“文件已入库后解析失败仍会 discard”，因此不满足 trivial 对
    “concurrency/lifecycle/runtime integration 无实质影响 + 极简验证”的定义，
    按 standard 的默认 bounded bugfix 流程执行（一份变更记录 + 独立缺陷发现
    的 completion-report）。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认」，以 standard
  终值批准，随后按 lower-tier 流程执行（pre-edit 基线 -> 实现 -> 验证 ->
  变更记录 -> completion-report）。

## Background and Goal

- 现象（中危缺陷）：`server/src/versions/http.rs` 上传 PUT handler 中，文件流先经
  `ingest_task` 写入并落库；随后若 body frame 读取失败、非 data frame（trailer）
  被拒、`parser.feed`/`parser.finish` 校验失败，handler 在 `joined` 已返回
  `Ok(file)` 的情况下直接 `return 422`，没有执行 `files.discard`。
- 已入库但从未发布成功的 files 行与磁盘文件成为孤儿；畸形 multipart 分帧或
  中途断流可持续制造孤儿，直至手工/GC 清理。
- 目标：上传 handler 在所有“文件已成功入库但请求最终被判定失败”的路径上都
  执行 `files.discard(&file.file_id)`，再返回 422；原有成功路径与既有
  sha256 不匹配、publish 失败的 discard 行为保持不变。

## Scope

### In scope

- 修复 `server/src/versions/http.rs` 上传 handler：当 `upload_error` 存在且
  `ingest_task` 结果已经是 `Ok(file)` 时，先 discard 再返回 422；
- 新增 HTTP 集成回归测试：构造“文件 part 完整写入后在后续分帧/结束边界校验
  失败”的畸形 multipart 上传，断言返回 422 且 `files` 表不残留孤儿记录。

### Out of scope

- 不修改 multipart 解析器本身（`server/src/versions/upload.rs`）与存储层
  `discard`/`gc_orphans` 行为；
- 不新增全库级孤儿清理机制或后台任务；
- 不改动 `ingest_task` join 失败的路径（任务 panic 时无法取得 file_id，仍按
  现有 500 语义返回）；
- 不处理版本已引用文件的正常清理语义（该路径已有 discard）。

### Boundary with neighboring modules

- 仅 filehub-server versions -> storage 的既有调用面变化：失败路径新增一次
  `FileStore::discard` 调用，storage 层无需改动。

## Requirement Review

- 需求合理：缺陷定位明确（http.rs 中 upload_error 早退分支缺少
  `joined == Ok(file)` 时的清理），修正是对现有 sha256 不匹配/publish 失败
  分支已有的 discard 语义的补齐；
- 材料风险/权衡：无公开契约变化；discard 失败（如 DB/IO 异常）沿用现有
  `let _ = ...` 忽略语义，不改变响应码；`upload_error` 与 `joined == Err`
  同时存在时仍优先返回 422（保持现状，不吞掉解析错误）；
- 选择方向：最小改动——在现有 422 早退分支内对 `joined` 做
  `if let Ok(file) = &joined { let _ = files.discard(&file.file_id).await; }`。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-upload-orphan-discard | 上传解析/trailer/结束边界失败且文件已入库时，先 discard 再返回 422 | 仅 http.rs 上传 handler；不触碰 upload.rs、存储层与版本服务 | discard 失败仍忽略并照常返回 422；优先级不变 | 新增集成测试复现“文件已落库后解析失败”，断言 422 且 files 表无孤儿行；全量 server 单测/集成测试通过 | 不改 GC/后台清理、不改解析器、不改变成功路径 |

## Success Criteria

- Concrete user-visible or system-visible result: 畸形/中断的上传请求返回 422
  后不再残留孤儿 files 行与未引用磁盘文件。
- Required evidence:
  - 一条新增 HTTP 集成回归测试（red -> green：修复前残留孤儿、修复后无残留）；
  - `cargo test -p filehub-server` 相关用例全绿；
  - `docs/changes/031-discard-orphan-upload.md` 与任务包
    `completion-report.md` 记录变更与独立缺陷发现结论。
- Explicit non-goals: 不引入后台孤儿清理任务；不改变 422 错误语义与响应体。

## Risks

- 低：新增 discard 调用在失败路径会多一次 DB/文件删除 IO；失败时继续按现状
  返回 422 并忽略清理错误，不会把清理失败升级为 500；
- 无 schema/迁移、无公开契约、无依赖变化；回归测试覆盖该失败分支。
