---
task_manifest: task.yaml
status: approved
---

# 上传路径鉴权前置、流式收流与 sha256 完整性校验（026 需求修订）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Requirement Revision Record

- 2026-08-25：用户确认方向「不需要解压校验，可以在上传协议中上报压缩包的
  sha256 hash，服务端校验下 hash 就 ok」。
- 2026-08-25（续）：用户确认上传时 `sha256` 字段必填；admin-web 需要在
  浏览器端计算所选文件的 sha256 并随 multipart 上报，服务端缺失/不匹配
  均拒绝。
- 原 P-03「有界解压 / 压缩炸弹拒绝」（change_id
  `fh-upload-size-and-decompression-bounds`）被本修订替换为
  `fh-upload-size-and-sha256-integrity`：服务端不再做任何 gzip/tar 解压校验，
  完整性完全由「上传协议携带压缩包 sha256 + 服务端计算并核对」承担。
- proposal 状态置为 draft，等待用户对修订后范围与层级确认；在确认前不继续
  执行 design/implementation/testing/acceptance 阶段的项目改动。
- 本修订不改变 P-01（授权前置）与 P-02（流式收流 / 内存 O(1) 上限）的
  已批准行为。

## Previous Approval Record

- approver: 用户
- approval_date: 2026-08-24
- user_statement: 用户回复「确认，自动完成」，批准 high-risk 提案并要求自动完成全流程。
- 该批准对应的原始范围以修订后的本文件为准；原始 P-03 的有界解压要求被
  本修订取代。

## Approval Record（本次修订）

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认，standard」，并确认上传时 `sha256` 必填；
  批准以 standard 层级按 lower-tier 流程完成实现、验证、变更记录与收尾。

## Workflow Tier Judgment

- Proposed tier: high-risk（维持原任务层级）
- Final tier: standard
  （2026-08-25 用户确认，层级由 high-risk 修订为 standard）
- Tier rationale / triggered boundaries:
  - 本修订属于已确认 high-risk 任务 026 的需求变更：将公开上传协议中的
    `sha256` 字段从可选收紧（推荐必填，见 Open Questions）、移除并替换一项
    完整性与容量控制（解压校验），跨越 server 与 admin-web 两个交付面，并
    涉及依赖与配置收敛；
  - 变更仍保留原任务已确认的实质影响面（授权时序、流式 multipart、存储
    容量边界），因此维持 high-risk 提案；若用户明确选择 standard，则按
    lower-tier 流程执行本修订的交付，剩余风险（协议字段必填、无格式校验的
    数据质量边界）在交付文档中显式记录。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认，standard」。

## Background and Goal

- 现象与决定：上一轮验收评审指出，现行 `max_decompressed_bytes` 只统计
  tar entry 的文件内容，tar header、padding 与扩展元数据由
  `tar::Archive` 直接消费、不经过 LimitedSink，无法真正封顶整体 gzip
  解压量；高压缩比的空文件/元数据 tar 可绕过限制消耗服务器 CPU。用户
  据此决定：服务端不需要解压校验，完整性校验改由上传协议上报压缩包
  sha256 承担。
- 现状：`FileStore::ingest` 已边收流边对压缩字节计算 sha256 并写入
  `files.sha256`；`versions/http.rs` 已读取 multipart `sha256` 字段并与
  入库记录比对（不匹配即 discard + 422）；CLI `publish_app` 已固定上报
  sha256，admin-web `uploadApp` 目前不上报。
- 目标：删除 `server/src/storage/integrity.rs` 与全部服务端 gzip/tar
  解压调用、相关配置与依赖；sha256 作为服务端唯一完整性校验并（推荐）
  必填；`max_archive_bytes` 流式计数限制、授权前置、流式请求体与下载
  原始字节语义全部保持不变；压缩炸弹 CPU 放大类攻击面随解压路径整体消失。

## Scope

### In scope

- 服务端存储与装配（`fh-upload-size-and-sha256-integrity`）：
  - 删除 `server/src/storage/integrity.rs`（`validate_targz`、LimitedSink）；
  - `SqliteFileStore` 移除 `max_decompressed_bytes` 字段与 `spawn_blocking`
    解压校验步骤，`FileModule::init` / `AppState::assemble` 同步移除参数与
    默认倍数推导；
  - `FilesConfig` 移除 `max_decompressed_bytes` 可选字段，
    `server/config.example.json` 移除对应配置；已部署配置中出现该键时
    serde 默认忽略，不破坏启动；
  - `server/Cargo.toml` 移除 `flate2`/`tar` 运行时依赖（测试夹具仍需要的
    话迁移到 dev-dependencies）。
- 上传协议与完整性校验（`fh-upload-size-and-sha256-integrity`）：
  - `PUT .../apps/{app}` 的 multipart `sha256` 字段必填：缺失/为空/不是
    64 位 hex 返回 422，保留现有不匹配 discard + 422 语义，严格保留现有
    服务端计算 sha256 与字段比对逻辑；
  - admin-web `uploadApp` 在发布前用 Web Crypto 计算所选 Blob 的 sha256
    并随 multipart 上报。
- 测试与文档：删除 `validate_targz` 压缩炸弹单测与集成断言，替换为
  「缺 sha256 422 / sha256 不匹配 422 且无残留、归档超限 422、合法上传
  回归」；`docs/api/v1-contract.md` 与 `docs/modules/filehub.md` 移除解压
  上限说明并记录 sha256 语义；026 的 design/testing 文档在确认后的对应
  阶段同步修订。

### Out of scope

- 不修改 CLI：`publish_app` 已固定上报 sha256；
- 不新增 gzip 魔数、文件名后缀、压缩比等任何格式/内容校验（服务端按
  不透明压缩字节存储）；
- 不做服务端解包、提取或内部文件级校验，不改变下载为原始字节流的语义；
- 不改变 `max_archive_bytes` 实时计数、Content-Length 预检、授权前置与
  multipart 解析行为；
- 不修改数据库 schema（files.sha256/size 语义不变）。

### Boundary with neighboring modules

- server 存储层不再依赖 flate2/tar；CLI（打包/解包方）仍保留自身依赖与
  sha256 生成/校验；
- admin-web 仅在用户确认 sha256 必填时新增浏览器端哈希计算，不改变其余
  UI 与接口调用面；
- 版本服务 `publish_app` 与下载流不受影响。

## Requirement Review

- 需求合理：服务端从不消费解压内容（入库只存压缩字节，下载原样流式返回），
  解压校验的唯一作用是「上传时确认结构可解析」。既然消费者是下载方且官方
  CLI 在发布前已自检打包，删除服务端解压校验并用压缩包 sha256 校验传输
  完整性，比修补计数器更彻底：CPU 放大攻击面整体消失。
- 材料风险与权衡：
  - sha256 只证明「收到的字节与上报哈希一致」，不能证明内容是合法 tar.gz；
    服务端契约变为不透明字节存储，损坏/非 tar.gz 上传可能在下载方解包时才
    暴露（官方 CLI 打包路径已排除大部分风险）；
  - sha256 不是真实性问题：发布者可对任意字节计算哈希后上传，但这不构成
    服务端 CPU/存储放大的新攻击面（存储与带宽仍受 `max_archive_bytes`
    约束）；
  - 已确认 `sha256` 必填，admin-web 等全部发布面都必须经过完整性校验；
    代价是 admin-web 需在浏览器端计算哈希（选中文件后立即计算，避免阻塞
    上传点击）。
- 选定方向：删除解压路径与配置 -> 收紧密封 sha256 上传语义 -> admin-web
  补齐哈希 -> 移除依赖/测试/文档中的解压内容。错误码保持现有 401/403/422
  契约不变（仅补「缺少 sha256」422）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-01 | fh-upload-authz-gate | PUT 上传路由在读取 body 前完成 `Resource::Project` + `artifacts:write` 判定；匿名 401、越权 403（行为保持不变） | 只收敛到上传路由，`publish_app` 内校验保留为纵深 | 与已确认实现一致，不因本修订回退 | 匿名/无权限超大 body 仍返回 401/403 且未进入解析 | 不批量迁移其它写路由 |
| P-02 | fh-http-body-limit | sfo-http 0.8 流式 body、增量 multipart、实时大小上限与 Content-Length 预检（行为保持不变） | 仅 actix-web 后端验证 | 与已确认实现一致，不因本修订回退 | 超大 chunked 请求仍在中段被 422 终止 | 不切换 HTTP 后端 |
| P-03 | fh-upload-size-and-sha256-integrity | 删除全部服务端解压校验与 `max_decompressed_bytes` 配置/依赖；`sha256` 必填并在 ingest/HTTP 层与流式计算哈希核对；admin-web 上报哈希 | 仅 server+admin-web；不改变 storage 存储/下载字节语义 | 不再有格式级提前校验；换取解压 CPU 攻击面消失 | `filehub-server` 无 flate2/tar 运行时依赖；无 sha256 字段 422 且无残留；sha256 不匹配 422 且 discard | 不做任何解压/格式校验替代品 |
| P-04 | fh-upload-security-tests | 更新上传安全与完整性测试：缺 sha256、sha256 不匹配、归档超限、压缩炸弹用例移除、合法上传/下载/哈希回归 | server 单测/集成 + admin-web 单测（仅哈希计算） | 用「缺少/错误哈希」替代「压缩炸弹」作为负向证据 | `cargo test -p filehub-server` 与 admin-web 相关测试全绿 | 不新增解压相关测试路径 |

## Success Criteria

- 可见结果：上传不带 `sha256` 或哈希不匹配返回 422 且不落库、无文件残留；
  服务端代码中不存在 gzip/tar 解压调用与解压上限配置；合法上传/更新/
  下载与 sha256 记录完全不变量。
- 必要证据：
  - `server/src/storage/integrity.rs` 删除；`store.rs` 无
    `spawn_blocking` 解压校验；`FilesConfig`/`AppState`/`FileModule` 无
    `max_decompressed_bytes`；
  - 集成测试覆盖缺失/错误 sha256 的 422 与 discard，压缩炸弹用例移除；
  - `cargo test -p filehub-server` 全绿；admin-web 哈希计算用例（若必填）
    + 端到端上传通过；
  - `docs/api/v1-contract.md` 与 `docs/modules/filehub.md` 移除解压上限、
    记录 sha256 语义；026 design/testing 文档按阶段同步。
- 明确非目标：不做格式/内容校验替代、不新增服务端解包能力、不改 DB schema、
  不改 CLI 发布/下载行为。

## Risks

- 公开契约变化：`sha256` 必填后，老 admin-web 版本上传将收到 422；本仓库
  处于 v0.1 内同批发布，server 与 admin-web 同步升级，并在
  `docs/api/v1-contract.md` v1 破缺说明中记录。
- 数据质量边界：服务端不再提前识别非 tar.gz 内容；契约更新为不透明压缩
  字节存储，下载方解包失败属发布方责任；官方 CLI 打包路径保留自身安全
  检查。
- 配置兼容：已配置 `max_decompressed_bytes` 的部署在代码移除字段后该键被
  serde 忽略（FilesConfig 无 deny_unknown_fields），不会启动失败；文档与
  示例配置同步删除。
- 依赖收敛：`flate2`/`tar` 若测试夹具仍需要则保留为 dev-dependencies，
  生产二进制不再携带；CLI 依赖不受影响。

## Material Assumptions and Tradeoffs

- `sha256` 已确认必填：完整性校验对所有发布面生效；admin-web 用 Web
  Crypto 计算整文件哈希（内存占用等于所选文件大小，默认 100 MiB 上限下
  可接受，超大文件需引入流式 WASM 哈希，超出本轮范围）。
- 删除解压校验不改变 `max_archive_bytes` 对压缩字节的磁盘/带宽上限；CPU
  不再存在解压放大路径。
- 服务端仍为下载方提供 `sha256` 字段，下载客户端（CLI）自行校验，行为不变。

## Open Questions

1. `sha256` 上传字段：已确认必填（2026-08-25 用户决定），不再开放。
2. 层级：已确认 standard（2026-08-25 用户回复「确认，standard」），
   按 lower-tier 流程执行；剩余风险（契约字段收紧、无格式校验的数据质量
   边界）在本任务变更记录与 completion-report 中显式记录。
