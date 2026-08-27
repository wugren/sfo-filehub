# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/036-require-upload-file-part.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `MultipartParser::finish()` 完成校验从“只检查结束边界”扩展为同时要求阶段
    完成、`seen_file` 为真且 `file_bytes > 0`：只有 `sha256`、没有 `file` part
    的 multipart 返回 `multipart body missing required file part`，显式 0 字节
    `file` part 返回 `multipart file part is empty`，HTTP 侧沿既有失败路径返回
    422 并 discard 已入库临时文件（031 语义），不再发布空 artifact；
  - `server/tests/unit/upload.rs` 新增两个解析器单测（任意分块宽度下缺 file /
    空 file 均在 finish 失败）；`server/tests/api_integration.rs` 新增集成回归，
    断言 422、版本 `apps` 为空、`files` 表无记录且 `data_dir` 无 `.tar.gz` 或
    `.tmp-` 残留；
  - 修复前 red 证据：还原旧 finish 后，仅 sha256 的 PUT 实测返回 201 并发布
    空 artifact，HTTP 回归断言在 201 vs 422 处失败；恢复修复后同名测试转绿。
- Handoff: `cargo test -p filehub-server` 全量通过（unit_tests 43/43、
  api_integration 5/5、dv_tests 2/2，多轮整包并行复跑确认稳定）；交付仅涉及
  `server/src/versions/upload.rs`、`server/tests/unit/upload.rs`、
  `server/tests/api_integration.rs`、`docs/changes/036-require-upload-file-part.md`
  与本任务包文档；未运行仓库级格式化，未触碰工作区其它未提交在制内容。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-upload-require-file-part | `finish()` 必须校验 `seen_file`，缺 file part 的上传在解析完成阶段报错 | proposal.md P-001 | `upload.rs` finish 阶段校验；缺 file 单测与 HTTP 回归 422、`apps` 为空 | 匹配 | pass |
| fh-upload-reject-empty-file | 按“不支持发布空文件”结论，`finish()` 拒绝 `file_bytes == 0` 的空 file part | proposal.md P-002 | `upload.rs` 空文件校验；空 file part 单测与 HTTP 回归 422、无磁盘残留 | 匹配 | pass |
| fh-upload-file-part-tests | 新增解析器单测与仅 sha256 无 file 的 HTTP 回归，且既有上传用例不回归 | proposal.md P-003 | `unit_tests` 43/43、`api_integration` 5/5、`dv_tests` 2/2 全绿 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `upload.rs` 状态机 finish 路径、Headers/Content 对 file 与 field 的分流、`http.rs` 失败路径与 discard 时序 | 反向推演：file 在 sha256 之后出现、任意分块切分、重复 file/重复 sha256、缺 file 但多字段、空 file 后跟正常 part，是否全部在 finish 阶段被拦截 | 只有结束边界校验时四种畸形输入均可通过；新增 seen_file 与 file_bytes 校验后按结构错误拒绝，正常 file+sha256 顺序无关仍成功 | pass |
| boundaries-and-failure-paths | 0 字节边界、唯一合法空流哈希、ingest 对空流的落库、discard 对行与磁盘文件的清理（031 用例） | 尝试空流在 ingest 已完成落库后失败，验证 422 后 files 行与 .tar.gz 均被清理；检查 Content-Disposition 缺 name 等既有解析错误未被吞掉 | 集成回归实测 files 计数 0、data_dir 无归档与 .tmp- 文件；空 file 失败路径与 sha256 缺失/不匹配路径共用既有一致的 422+discard 语义 | pass |
| regression-and-side-effects | 全量 test 产物、上传安全边界、孤儿清理、版本输入校验用例、变更清单与工作区未提交文件 | 核对新校验是否改变成功上传、锁版本、下载、sha256 缺失/不匹配语义；核对临时还原修复不会遗留中间状态；核对未改动 CLI/admin-web 与其它在制文件 | unit_tests 43/43、api_integration 5/5、dv_tests 2/2；多轮整包并行复跑稳定；变更清单与 red-to-green 证据一致，工作区其余在制内容未被触碰 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests
  parser_rejects_missing_file_part parser_rejects_empty_file_part` 与
  `cargo test -p filehub-server --test api_integration
  upload_rejects_missing_or_empty_file_part` 通过；临时还原旧 finish 后同一组
  回归 red（201 发布空 artifact），恢复修复后转绿
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 一次整包并行运行时 `api_login_session_and_token_flow` 登录首发返回 502 | 该用例与上传链路无关（未触碰 login 路径），隔离复跑 3/3、后续整包并行连续 4 轮 5/5 全绿，判定为环境/装载偶发；如需根治可另立测试基建任务 | no |
| F-2 | low | 空 file part 在 finish 阶段才失败，0 字节文件在 ingest 中短暂存在 | 失败后由既有 discard 清理，实测无残留；从“先收流后校验”的流式设计看属预期窗口，不构成本次交付缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001 至 P-003 全部落地：缺少 file part 与 0 字节空 file 均在
  finish 阶段被拒绝，HTTP 回归实测修复前发布空 artifact、修复后 422 且无任何
  残留；解析器单测与集成回归齐全，全量 43+5+2 用例绿；独立缺陷发现覆盖行为
  逻辑、边界失败路径与回归副作用，F-1/F-2 均无阻塞。
