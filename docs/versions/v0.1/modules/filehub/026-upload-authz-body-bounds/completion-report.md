# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/026-upload-authz-body-bounds.md

## Delivery Summary
- Outcome:
  - 服务端删除全部 gzip/tar 解压校验（`integrity.rs` 删除、`store.rs`
    `spawn_blocking` 校验移除），`max_decompressed_bytes` 配置、AppState/
    FileModule/SqliteFileStore 字段与默认倍数推导一并移除；
  - 上传协议 `sha256` 必填：`versions/http.rs` 对缺失/非 64 位 hex 返回
    422（已入库文件先 discard），与流式计算哈希不一致同样 422 + discard；
  - admin-web `sha256.ts`（Web Crypto）+ `ProjectDetailPage` 上传前计算并
    上报哈希，`client.uploadApp` 的 `sha256` 参数改为必填；
  - `flate2`/`tar` 移入 server dev-dependencies；契约/模块文档同步；
  - 测试改造：删除压缩炸弹用例，新增“任意非 gzip 字节按不透明流入库”
    “缺少 sha256 422”“sha256 不匹配 422 且版本未发布”回归，既有成功上传
    用例补齐必填哈希。
- Handoff: `cargo test -p filehub-server` 全量 39 项通过（4 api_integration
  + 2 dv + 33 unit）；`admin-web` unit 44 项通过、`npm run build` 通过；
  变更清单经 pre-edit/completion baseline 对比只含本任务增量。

## Proposal Consistency
| Change ID | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-upload-authz-gate | 授权前置行为保持不变：PUT 上传在读取 body 前 401/403 | proposal.md P-01 | authz 与路由未改动；api_integration 匿名 1MiB 脏 body 仍 401 | 匹配 | pass |
| fh-http-body-limit | sfo-http 0.8 流式 body、增量 multipart、实时大小上限保持不变 | proposal.md P-02 | upload.rs 未改动；upload_security_boundaries 归档超限 422 与任意切分单测通过 | 匹配 | pass |
| fh-upload-size-and-sha256-integrity | 删除解压校验与配置/依赖；sha256 必填并核对；admin-web 上报哈希 | proposal.md P-03 | integrity.rs 删除；store/http/config 无 max_decompressed 引用；is_sha256_hex + discard；sha256.ts/ProjectDetailPage 上报 | 匹配 | pass |
| fh-upload-security-tests | 缺少/非法/错误 sha256 422 且无发布；删除压缩炸弹断言；契约/模块文档更新 | proposal.md P-04 | upload_security_boundaries 缺 hash/wrong hash 422 且 apps 为空；upload_ingest/storage 新断言；docs/api/v1-contract.md 与 docs/modules/filehub.md 更新 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `versions/http.rs` 上传 handler 的 sha256 分支与 discard 顺序、`store.rs` 移除校验后的 ingest 流程、`client.ts` form 组装、`sha256.ts` 实现 | 缺失字段/空值 part（parser 已拒）/65 位 hash/大小写 hash/文件流失败与哈希失败并存时清理顺序；服务端 64 hex 与 `eq_ignore_ascii_case` 一致 | 缺失或非法 sha256 只在 joined 成功并 discard 后 422；opaque ingest 单测确认不再解压；无遗漏分支 | pass |
| boundaries-and-failure-paths | `upload_security_boundaries`（匿名 401、归档超限、缺 hash、错 hash、apps 为空断言）、`upload_parse_failure_after_ingest_discards_orphan`、ingest sha 不匹配单测 | 无 file part、空 sha256 part、超长 hash 字段（parser max_field_bytes）、错 hash 与 publish 失败并存 | 失败路径均 422 且不发布；缺 hash 复用 031 的 discard 语义；边界均在 parser/handler 双侧断言 | pass |
| regression-and-side-effects | 全量 `cargo test -p filehub-server`、admin-web unit+build、docs/api/v1-contract.md 与 docs/modules/filehub.md diff、Cargo.lock diff | 生产依赖不再含 flate2/tar（cli 保留）；旧配置含 max_decompressed_bytes 被 serde 忽略；既有上传客户端均补 sha256；baseline diff 无范围外文件 | 39 + 44 项测试与 tsc/vite 构建全绿；server 生产二进制解压栈移除；CLI/web 行为无回归 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server`（4 api_integration + 2 dv +
  33 unit）；`admin-web` `npm run test:unit`（44 项，含 sha256.test.ts）与
  `npm run build`（tsc + vite）
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | medium | 服务端不再做任何格式/内容校验 | 损坏或非 tar.gz 上传只会在下载方解包时暴露（官方 CLI 自带打包校验，契约已声明服务端按不透明字节存储）；属于需求修订接受的契约边界，非缺陷 | no |
| F-2 | low | `sha256.ts` 使用 `file.arrayBuffer()` 整文件哈希 | 浏览器内存峰值约等于所选文件大小，受默认 `max_archive_bytes`（100 MiB）约束；超大归档需流式 WASM 哈希，本轮非目标 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 需求修订（移除解压校验、sha256 必填）已按 standard 完整落地：全部
  服务端解压路径/配置/依赖清理、协议必填校验与 admin-web 哈希上报均实现，
  缺失/错误哈希 422 不发布，既有授权前置/流式收流/上限行为无回归；独立
  缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，未发现阻塞性缺陷；
  F-1/F-2 为需求修订明确接受的契约与容量边界。
