---
task_manifest: task.yaml
status: approved
---

# 限制匿名登录接口 /account/login 请求体最大 64 KiB

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，确认采纳提案
  （仅登录接口 64 KiB 上限：Content-Length 预检 + 实际流量有界读取 + nginx
  登录精确匹配路由限制）并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 修改点是 filehub 单一交付面（sfo-account 登录 handler + Docker nginx 模板），
    属于有界安全加固 bugfix：为未认证登录接口增加 64 KiB 请求体上限，需要新增
    HTTP 级回归测试验证 Content-Length 与真实流量两条路径；
  - 不满足 trivial：改变的是认证前输入边界（安全/边界行为变更），且需要确定性
    回归验证；与 026/036/037 等同类 body-bound 任务沿用同一 standard 判定；
  - 未触发 high-risk：不新增公开 API 字段或协议、不涉及数据 schema/迁移、直接
    依赖仅补齐锁文件内已有包（http-body-util、serde_json，均经 sfo-http 已在
    解析图中）、不改变发布/兼容/回滚语义；nginx 只对 /account/login 增加
    64 KiB 上游限制，其余上传/下载位置仍为 0 不受影响。
## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 回复「确认」，确认采纳本提案（登录请求体最大
  64 KiB，含 Content-Length 预检、实际流量有界读取与 nginx 登录路由 64k 限制）
  并接受建议的 standard 层级。

## Background and Goal

- 现象（评审第 2 项，高危）：`POST /account/login` 在认证前直接调用
  `req.body_json()`（`third_party/sfo-account/src/account_server.rs`），
  而 sfo-http 0.8 的 body 读取会把整个 body 无上限累积进 `BytesMut`
  （crates.io `sfo-http-0.8.0/src/actix_server/endpoint.rs` 的
  `body_bytes`/`body_string`）；Docker 侧 nginx 又设 `client_max_body_size 0`
  （`docker/nginx.conf`），未登录攻击者可并发发送超大/无限 chunked 登录请求，
  消耗 nginx 临时存储与服务端内存。
- 目标（用户确定的口径）：登录请求体长度上限固定为 64 KiB（65536 字节），
  同时约束 Content-Length 声明值和实际读入流量（含无 Content-Length 的
  chunked 流量）。

## Scope

### In scope

- `third_party/sfo-account/src/account_server.rs`：登录 handler 在 `body_json`
  前先做 Content-Length 预检（声明 > 64 KiB 直接拒绝），再改用有界流读取实际
  请求体（累计超过 64 KiB 立即拒绝并停止读取），最终由同一 JSON 解析路径解析；
  超限错误沿用 `AccountErrorCode::InvalidParam`；
- `third_party/sfo-account/Cargo.toml`：补充 `http-body-util`（0.1）与
  `serde_json`（1）直接依赖（均已在 Cargo.lock 中存在，无新增包）；
- `docker/nginx.conf`：新增 `location = /account/login` 精确匹配块并设置
  `client_max_body_size 64k;`（仅登录路由，`/api/v1/` 上传/下载不受影响）；
- `server/tests/api_integration.rs`：新增回归用例覆盖两条真实流量路径——
  声明 Content-Length 超过 64 KiB 的固定长度请求，以及不携带 Content-Length
  的流式（chunked）超限请求。

### Out of scope

- 不限制 `/account/get_account_info_of_session`、`/account/refresh_session`
  等其它 JSON 路由（用户口径限定为登录接口；残余风险见 Risks）；
- 不修改 sfo-http crates.io 源码、不为 sfo-http 增加本地 patch；
- 不修改上传/下载的 `client_max_body_size 0` 与 `FH_MAX_ARCHIVE_BYTES` 语义；
- 不修改登录协议字段、错误文案、`docs/api/v1-contract.md` 或客户端；
- 不触碰 025-037 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 登录超限错误仍走 AccountServer 既有 `InvalidParam` 结果序列化路径，
  HTTP 状态与错误结构不变；恰好 64 KiB 的合法 JSON 仍然进入账号校验；
- 有界读取基于 `sfo_http::http_server::Request::take_http_body` 这一 trait
  方法，actix 与 hyper 后端同时受约束，不针对单后端。

## Requirement Review

- 需求合理：登录体只需要三个小字段，64 KiB 上限远高于合法使用（用户名/密码/
  时间戳正常几十字节），能封住未认证超大/chunked 请求的内存与 nginx 临时存储
  消耗，且与评审建议口径一致（用户从「登录及普通 JSON 路由统一 64 KiB」收窄为
  「仅登录 64 KiB」）；
- 方向选择：应用层采用「Content-Length 预检 + 有界流读取」双重约束，比只检查
  头部更完整（chunked 无 Content-Length 时仍受实际流量约束）；nginx 层对登录
  路由精确匹配设置 64k，让 Docker 部署在代理缓冲前直接拒绝超大体；
- 材料风险/权衡：登录 handler 从 `body_json()` 改为自研有界读取 + `serde_json`
  解析，行为等价（错误码、消息结构不变），但为 sfo-account 增加两个已在解析
  图中的直接依赖；nginx 精确匹配块需与 `^~ /account/` 保持同一组反代头设置。
- 待确认问题：无。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-login-body-limit | 登录请求体最大 64 KiB：Content-Length > 65536 或实际累计读取 > 65536 均拒绝（InvalidParam） | 仅 `third_party/sfo-account` 登录 handler；错误走既有 JSON 结果结构 | 自研有界读取替代 `body_json()`，等价解析行为 | 65 KiB（固定长度与 chunked 两种形态）请求返回 InvalidParam；正常登录回归通过；恰好 64 KiB 合法 JSON 仍进入账号校验 | 不限制其它路由，不修改 sfo-http |
| P-002 | fh-login-body-limit-nginx | docker nginx 对 `location = /account/login` 设置 `client_max_body_size 64k`，保留相同反代头 | 仅 `docker/nginx.conf` 登录精确匹配块 | 代理缓冲前直接拒绝超大登录体；上传/下载不受影响 | `location = /account/login` 生效且反代行为与 `^~ /account/` 一致 | 不把 64k 上限扩到上传/下载或其他 account 路由 |
| P-003 | fh-login-body-limit-tests | HTTP 级回归：固定长度 65 KiB 与流式 chunked 超限登录均返回非 0 err | `server/tests/api_integration.rs` 登录用例区 | 覆盖 Content-Length 与真实流量两条边界路径 | 新增用例通过；`api_login_session_and_token_flow` 等既有登录集成用例不回归 | 不新增 sfo-account 单测或 mock Request 基建 |

## Success Criteria

- 65 KiB（含 65537 字节）固定长度登录体与 chunked 登录体均被拒绝，返回
  `AccountErrorCode::InvalidParam`（err != 0），过程中不累积超过上限的请求体；
- 恰好 64 KiB 的合法 JSON 仍进入账号校验（返回账号不存在/密码错误而非超限）；
- 既有正常登录集成用例（`api_login_session_and_token_flow`）继续通过；
- `cargo test -p filehub-server` 的相关集成用例与编译检查通过；
- docker nginx 模板中登录路由 64k 限制与反代头完整；上传/下载位置未改动；
- 按 standard 流程产出 `docs/changes/038-login-body-limit.md` 与任务包
  `completion-report.md`（中文正文），并经 lower-tier-check 校验。

## Risks

- 残余风险：`/account/get_account_info_of_session` 等其余 account JSON 路由
  应用层仍无 64 KiB 上限（用户口径收窄为仅登录）；将在交付文档中显式标注，
  不作为本任务范围；
- 依赖面：`http-body-util`、`serde_json` 直接依赖虽已在 Cargo.lock 的解析图内，
  但版本约束由 sfo-account 显式声明，构建图会新增两条直接边；
- sfo-account 是有意保持「仅提升 sfo-http 0.8 兼容」的 shim；登录读取逻辑定制
  后，后续上游版本同步时需要携带该安全改动；
- 工作树存在大量未提交的用户改动（025-037 等在制内容），本任务只修改提案列出
  的文件，不运行仓库级格式化；何时可全量测试视 036 在制用例状态而定。
