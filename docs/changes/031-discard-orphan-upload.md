# 上传解析失败后清理已入库孤儿文件

- Status: complete
- Owner module: filehub（filehub-server versions 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/031-discard-orphan-upload/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/031-discard-orphan-upload/proposal.md`
- Affected paths: `server/src/versions/http.rs`、`server/tests/api_integration.rs`、
  `server/Cargo.toml`、`Cargo.lock`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`server/src/versions/http.rs` 上传 handler 在 `ingest_task.await`
  之后先检查 `upload_error` 并直接返回 422；此时 ingest 可能已完成落库
  （`joined == Ok(file)`），却没有执行 `files.discard`，从而残留孤儿
  `files` 行与磁盘文件。畸形 multipart 分帧、trailer 帧或中途断流可反复触发。
- 最小修复：在 422 早退分支中对 `joined` 取引用判断
  `if let Ok(file) = &joined { let _ = files.discard(&file.file_id).await; }`，
  再返回 422。成功路径、sha256 不匹配分支、publish 失败分支的既有 discard
  语义与错误优先级均未改变；`ingest_task` join 失败（无法取得 file_id）仍按
  原 500 语义返回。
- 回归测试：`server/tests/api_integration.rs` 新增
  `upload_parse_failure_after_ingest_discards_orphan`——先以流式分帧发送完整
  file part（解析器停留在不完整头部，FileChunk 已返回并写入 ingest），第二帧
  补全空值 sha256 part 触发解析失败；断言 422 且重新打开同一 SQLite 后
  `files` 表计数为 0。修复前该用例失败（残留 1 条孤儿记录），修复后通过。
- 测试依赖：`server/Cargo.toml` dev-dependencies 新增 `futures-util = "0.3"`
  并为 `reqwest` 开启 `stream` feature（均只影响测试构建，Cargo.lock 无新增
  包）；Cargo.lock 随此同步，未改动任何生产依赖。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no（无 schema/迁移改动；修复
  复用既有 `files.discard` 清理语义）
- Security, privacy, or trust-boundary change: no（失败路径不再遗留可被
  GC/管理流程遗漏的孤儿文件与记录，接口响应与鉴权顺序不变）
- Concurrency, lifecycle, or runtime integration change: no（处理仍是 handler
  内单点 await，仅失败分支多一次 discard 调用；无新增并发/后台任务）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（`futures-util`/reqwest `stream` 仅为 dev-dependencies，
  生产二进制与锁文件包集合不变）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: 回归测试先 red（`files` 计数=1）后 green（计数=0）；
  `cargo test -p filehub-server` 全量 39 项通过（4 api_integration + 2 dv +
  33 unit）;
- Result: pass
- Residual risk or follow-up: `ingest_task` 自身 panic 时仍无法取得 file_id
  清理孤儿（保持现有 500 语义，属非阻塞后续项）；集成测试靠 200ms 分帧停顿
  保证跨帧失败路径，若未来服务端改为整包缓冲可退化为不触发该分支（测试仍
  通过但不侦测孤儿）。
