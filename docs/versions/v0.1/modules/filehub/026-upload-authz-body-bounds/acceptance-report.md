---
task_manifest: task.yaml
status: draft
---

# 上传鉴权前置与流式收流 验收报告

Risk profile: ./risk-profile.yaml

## Object and Scope

- Task manifest: task.yaml
- Review mode: independent（本轮由验收阶段独立于实现/测试的复查完成；环境不提供第二名 reviewer，按 acceptance-review-rules 的独立顺序执行并直接检查源码、测试与运行证据）

## Findings

| id | severity | owning_stage | correctness_category | evidence | problem | blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F1 | medium | design | interface-and-compatibility | 根 `Cargo.toml` 新增 `[patch.crates-io] sfo-account = { path = "third_party/sfo-account" }`（MIT shim，仅提升 sfo-http 0.7→0.8）；已批准 design.md 的 Directly Mapped Change Items Scope Paths 与 admission stamp scope paths 均不含根 Cargo.toml/third_party | 实现依赖上游 sfo-account 仍绑定 sfo-http 0.7，被迫引入仓库内第三方源码 shim；这是已批准设计与准入范围之外的构建面偏差，属于材料性依赖来源变更，需回设计补充 Scope Paths 与风险记录 | yes |
| F2 | low | implementation | boundary-and-input | `server/src/versions/upload.rs` 的 `closing_delim` 直接扫描 `\r\n--{boundary}`；若上传方选定与文件内容重复的 boundary，文件会在内容内被误判截断（随后 tar 校验 422 或按截断内容落库） | 边界碰撞只影响自选 boundary 的授权客户端（自伤类），随机 boundary 的既有客户端不受影响；作为残余风险记录，不在本任务修复 | no |
| F3 | low | testing | test-adequacy | 无 Content-Length 的 chunked 超限仅在单元层覆盖（`upload_ingest.rs` duplex 分块 + `upload.rs` 任意切分），未在真实 HTTP chunked 传输层面断言 | 真实 wire 级 chunked 中断属于次要缺口，测试层记录 gap，不阻塞本任务修复目标 | no |
| F4 | none | none | boundary-and-input | `versions/http.rs` 缺少 file part 时 ingest 收到空流并报 generic gzip 流错误 | 错误信息可读性改进项，无行为/安全影响 | no |

## Requirement Coverage

| change_id | requirement_or_boundary | source | implementation_evidence | finding | status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-upload-authz-gate | PUT 上传在读取 body 前完成 artifacts:write 判定；匿名 401、越权 403；`publish_app` 纵深校验保留 | proposal.md P-01；design/http.md | `server/src/http/authz.rs`（ProjectAuthzEndpoint）在 `versions/http.rs` 上传路由 body 读取前调用 `can_access`；匿名 1 MiB 脏 body 集成断言 401（api_integration.rs upload_security_boundaries） | 无 | pass |
| fh-http-body-limit | sfo-http 0.8 流式 body；增量 multipart 解析；实时大小上限与 Content-Length 预检 | proposal.md P-02；design/versions.md | `server/src/versions/upload.rs` MultipartParser（feed→Vec 多事件、Boundary/Closing 相位）；`versions/http.rs` take_http_body + duplex；任意切分与二进制保留单元测试全绿 | 无（F1 为独立记录的非本项功能偏差） | pass |
| fh-upload-size-and-decompression-bounds | ingest 流式入库实时计数；validate_targz 限量 sink；max_decompressed_bytes 可选配置 | proposal.md P-03；design/storage.md、model.md | `server/src/storage/store.rs` 流式 spool+sha+spawn_blocking 校验；`integrity.rs` LimitedSink/Chain 魔数；`model/config.rs` 可选字段；超限与压缩炸弹 422 断言 | 无 | pass |
| fh-upload-security-tests | 未授权/超限/压缩炸弹用例与契约文档对齐 | proposal.md P-04；design.md | `server/tests/unit/{upload,upload_ingest,storage,versions}.rs`、`api_integration.rs`（3 用例含 upload_security_boundaries）、`docs/api/v1-contract.md` 上传说明、testplan.yaml 4 步 task-all 全绿含 run artifact | 无（F3 记录测试层次要 gap） | pass |

## Independent Defect Discovery

| category | applicable_scope | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|------------------|--------------------|-------------------|-----------------------------------|--------|
| requirement-and-behavior | proposal 范围与评审第 2 项四条证据 | proposal.md Background（body_bytes/store.rs:48/integrity.rs:19/service.rs:211）、design.md、代码实现 | 逐条核对：授权前置、有界流式、解压封顶、顺序约束均落地；未发现范围外迁移或遗漏 | 无新增需求缺口；F1 属设计记录偏差 | pass |
| logic-and-control-flow | MultipartParser 相位机与 ingest 分支 | upload.rs feed/parse_one/Closing/Boundary、store.rs spool 循环、authz.rs | 对 226 种 chunk 切分与单/双 part、收尾 `--`/CRLF 切分做穷举式反例搜索 | 状态机无 off-by-one/死循环；progress/blocked 语义正确 | pass |
| boundary-and-input | 大小/边界/boundary 碰撞/字段类输入 | upload.rs 上限分支、api_integration upload_security_boundaries（1MiB 脏 body、超限、bomb） | 尝试 boundary 与内容重复、超大字段、总长度溢出、收尾尾随数据 | F2：boundary 与内容重复的自伤性截断（非阻断，残余风险）；其余边界全部拒绝 | fail |
| state-and-data-integrity | 临时文件/落库/版本引用 | store.rs 原子 rename、discard、gc；versions publish 失败 discard | 超限/校验失败/发布失败后的文件、DB 行与版本引用状态 | 失败路径清理与孤儿回收语义保持；sha/大小与文件一致 | pass |
| error-handling-and-recovery | 解析错误、上游流错误与入库错误映射 | versions/http.rs upload_error 优先、spawn_blocking join、ApiError 映射 | 流中断、写入关闭、join panic、解析错误与入库错误并存 | 错误处理后置校验并确保清理完成；F4 仅信息可读性 | pass |
| resource-lifetime-and-cleanup | 临时文件与任务句柄生命周期 | store.rs drop(file) 后 rename、spawn_blocking、duplex writer drop；0600 语义不涉及 | Windows rename 前释放句柄、超限删除、任务 detached 场景 | 临时文件无残留；句柄释放顺序保持 010 修复经验 | pass |
| concurrency-and-ordering | duplex 背压与并行 ingest | upload_ingest.rs 任意分块；versions/http.rs 事件顺序 | 写阻塞、读侧 EOF、事件乱序、并发上传隔离 | 无共享可变状态；背压与顺序由 duplex/顺序写入保证 | pass |
| interface-and-compatibility | 依赖源/导出 API/消费者 | design Consumer Migration Closure 五行；admission/e证与实际路径 | 检查 admitted scope 与实际改动全集 | F1：third_party/sfo-account 与根 Cargo.toml patch 未入已批准设计范围；迁移表其余行闭环 | fail |
| security-and-capacity | 未认证 DoS/压缩炸弹/授权边界 | authz wrapper、ingest cap、LimitedSink、MaxTotal；api_integration 匿名 401+bomb 422 | 匿名超大 body、解压放大、无界内存、5xx 掩盖 | 匿名 401 先于 body；内存 O(1)；解压封顶；未发现旁路 | pass |
| test-adequacy | 测试能否暴露 normal/boundary/negative/error 各类失败 | unit（upload/upload_ingest/storage/versions）、dv、api_integration 与 testplan 4 步 run artifact | 缺失断言?（真实 chunked wire、boundary 碰撞、缺 file part 文案） | F3：真实 HTTP chunked 无 Content-Length 场景仅单元覆盖；其余类别断言充分且可失败 | fail |

## Document Consistency

| document | source | implementation_consistency | finding | status |
|----------|--------|---------------------------|---------|--------|
| proposal.md | proposal.md | 上传目标/范围/成功标准与实现一致；P-01..P-04 闭环 | 无不一致；F1 属实现期偏离而非提案错误 | pass |
| design | design.md | 授权包装器、流式解析、流式 ingest、配置字段均按设计实现 | F1：dependency shim（third_party/sfo-account 与根 Cargo.toml patch）未写入设计 Scope Paths/Design Notes，构建面描述不完整 | fail |
| testing | testing.md, testplan.yaml | 用例与实现、文档一致；task-all run artifact 记录 6 条命令全绿 | F3 已记录；无其它漂移 | pass |

## Result Summary

- Overall result: needs changes
- Outcome: 功能目标（授权前置、流式收流、解压封顶、422/401 语义）经代码与测试验证达成；发现 1 项材料性设计范围偏差与 2 项非阻断残余风险。
- Blocking issues: F1（设计范围未覆盖 sfo-account 0.8 兼容 shim 的依赖源变更）
- Next action: 返回 design 阶段，把 third_party/sfo-account 与根 Cargo.toml patch 写入设计 Scope Paths 与 Design Notes/构建风险，重新批准并重跑准入与全量验证后回到验收。

## Conclusion

- Accepted / rejected / needs changes: needs changes
- Reason: F1 是已批准设计与准入范围之外的材料性构建面偏差；approved intent（sfo-http 0.8 流式上传）有效，回 design 补齐依赖源范围与风险记录后即可闭环；F2/F3/F4 为非阻断记录。
