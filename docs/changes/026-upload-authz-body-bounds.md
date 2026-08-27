# 移除服务端解压校验，上传协议 sha256 必填

- Status: complete
- Owner module: filehub（filehub-server storage/versions/http + filehub-web）
- Task manifest: `docs/versions/v0.1/modules/filehub/026-upload-authz-body-bounds/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/026-upload-authz-body-bounds/proposal.md`
- Affected paths: `server/src/storage/integrity.rs`（删除）、
  `server/src/storage/{mod,store}.rs`、`server/src/http/mod.rs`、
  `server/src/model/config.rs`、`server/src/versions/http.rs`、
  `server/config.example.json`、`server/Cargo.toml`、`Cargo.lock`、
  `admin-web/src/api/{client,sha256}.ts`、`admin-web/src/pages/ProjectDetailPage.tsx`、
  `server/tests/common/mod.rs`、`server/tests/api_integration.rs`、
  `server/tests/unit/{upload_ingest,storage}.rs`、`admin-web/tests/unit/{client,sha256}.test.ts`、
  `docs/api/v1-contract.md`、`docs/modules/filehub.md`
- Explicit tier override: none（用户 2026-08-25 确认 standard，记录于提案 Approval Record）
- Expanded high-risk packet: existing task packet 026（由 high-risk 需求修订降为
  standard；历史分期文档保留为存量记录，不以本次交付重复签发）

## Approach

- 需求来源：验收评审指出 `max_decompressed_bytes` 只统计 tar entry 内容、
  tar header/padding/扩展元数据不经 LimitedSink，无法真正封顶整体解压量；
  用户决定服务端不再解压校验，改由上传协议必填并核对压缩包 `sha256`。
- 服务端清理：
  - 删除 `server/src/storage/integrity.rs`（`validate_targz`/LimitedSink）；
  - `SqliteFileStore::new`/`FileModule::init`/`AppState` 移除
    `max_decompressed_bytes` 字段、参数与默认倍数推导；
  - `FilesConfig` 与 `config.example.json` 移除可选配置（旧配置含该键时
    serde 忽略，不阻塞启动）；
  - `store.rs` 删除 `spawn_blocking` 解压校验步骤，入库只保留流式
    `max_archive_bytes` 计数、流式 sha256 计算与 `expected_sha256`（可选）
    核对；
  - `server/Cargo.toml` 将 `flate2`/`tar` 从生产依赖移至 dev-dependencies
    （测试夹具仍构造真实 `.tar.gz`），CLI 依赖不受影响。
- 上传协议：`PUT .../apps/{app}` 的 multipart `sha256` 字段必填——缺失、
  非 64 位 hex 或与入库记录哈希不匹配均 422，且已入库文件先
  `files.discard`；校验函数为 `versions/http.rs::is_sha256_hex`。
- admin-web：新增 `src/api/sha256.ts`（Web Crypto `crypto.subtle.digest`），
  `ProjectDetailPage` 上传前计算所选 Blob 的 sha256 并随 `uploadApp` 上报；
  `client.ts` 将 `sha256` 参数由可选改为必填并始终 append。
- 测试与文档：删除压缩炸弹用例；新增“任意非 gzip 字节按不透明流入库”
  与“缺少 sha256 422、sha256 不匹配 422 且版本未发布 app”回归；全部既有
  成功上传用例补充正确 `sha256`；`docs/api/v1-contract.md`、
  `docs/modules/filehub.md` 移除解压上限说明并记录必填语义。

## Risk Screen

- Public contract, protocol, or CLI change: yes —— `sha256` 由可选变必填，
  admin-web 同步实现并上报；CLI 本就必带，无需改动；v1 契约文档已更新
- Persistent data, schema, or migration change: no（files 表 sha256/size
  语义不变，无迁移）
- Security, privacy, or trust-boundary change: yes —— 移除服务端 gzip/tar
  解压路径，压缩炸弹 CPU 放大攻击面整体消失；完整性改由上传必填 sha256
  承担，服务端不再做内容格式校验（残余风险见 Verification）
- Concurrency, lifecycle, or runtime integration change: no（删除
  `spawn_blocking` 校验线程，其余时序不变；失败路径 discard 语义沿用）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes —— `flate2`/`tar` 从 server 生产依赖移入
  dev-dependencies，生产二进制不再携带解压栈；Cargo.lock 同步；CLI 与
  web 构建产物不受影响
- Material UI, accessibility, localization, or navigation workflow change: yes
  —— admin-web 上传按钮在发布前多一步浏览器哈希计算（用户不可见耗时），
  无 UI/文案/流程形态变化
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: yes —— server 与 admin-web
  同一 filehub 模块的两个交付面同步执行协议变更，存储层契约变为“不透明
  压缩字节 + 必填 sha256”

## Verification

- Targeted check: `cargo test -p filehub-server` 全量 39 项通过（4
  api_integration + 2 dv + 33 unit）；`admin-web` `npm run test:unit` 44 项
  通过（含新增 sha256.test.ts）；`npm run build`（tsc + vite）通过
- Result: pass
- Residual risk or follow-up: 服务端不再校验内容格式，损坏或非 tar.gz 上传
  会在下载方解包时暴露（官方 CLI 发布路径自带打包校验；契约文档已声明）；
  浏览器端整文件哈希内存占用约等于所选文件大小，受默认 100 MiB 归档上限
  约束，超大文件需引入流式 WASM 哈希（本轮明确为非目标）。
