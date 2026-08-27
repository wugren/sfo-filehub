# 匿名 session 信息接口 /account/get_account_info_of_session 请求体上限 64 KiB

- Status: complete
- Owner module: filehub（sfo-account session 信息 handler + docker nginx 模板 + HTTP 集成测试）
- Task manifest: `docs/versions/v0.1/modules/filehub/045-session-body-limit/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/045-session-body-limit/proposal.md`
- Affected paths: `third_party/sfo-account/src/account_server.rs`、
  `docker/nginx.conf`、`server/tests/api_integration.rs`、
  `docs/changes/045-session-body-limit.md`、
  `docs/versions/v0.1/modules/filehub/045-session-body-limit/completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷：`POST /account/get_account_info_of_session` 在校验 session 前调用
  sfo-http 0.8 的 `body_json()`，该实现会把整个 body 无上限累积进 `BytesMut`；
  docker nginx 的 `client_max_body_size 0` 作用于整个 `^~ /account/`，038 仅对
  `/account/login` 增加了 64k 限制，未登录攻击者可继续对
  get_account_info_of_session 发送超大/持续 chunked body 消耗 nginx 临时存储与
  服务端内存。
- 修复（应用层）：把 038 的 `read_login_body()` 泛化为共享
  `read_bounded_json_body(req, route)`——先解析 `Content-Length` 预检（声明 >
  65536 字节直接拒绝），再用 `take_http_body()` 流式读取并累计，超过 64 KiB
  立即拒绝；`read_login_body` 保留「login」文案，session 信息接口经
  `read_session_info_body` 使用同一条 64 KiB 上限；超限错误沿用
  `AccountErrorCode::InvalidParam`。该读取基于 `sfo_http::Request::take_http_body`
  trait，actix 与 hyper 后端同时受约束。
- 修复（代理层）：`docker/nginx.conf` 新增
  `location = /account/get_account_info_of_session` 精确匹配块并设
  `client_max_body_size 64k`，反代头与 `^~ /account/` 一致；Docker 部署在代理
  缓冲前直接拒绝超大体；`/api/v1/` 上传/下载及全局 0 不受影响。
- 依赖：无新增直接依赖（`http-body-util`、`serde_json` 已由 038 加入
  `third_party/sfo-account/Cargo.toml`）。
- 测试：`server/tests/api_integration.rs` 新增
  `get_account_info_of_session_rejects_request_body_over_64k`，覆盖恰好 64 KiB
  合法 session JSON 仍进入解签、固定长度 65 KiB Content-Length 预检、无
  Content-Length 的流式 chunked 实际流量超限三条路径。

## Risk Screen

- Public contract, protocol, or CLI change: no（请求/响应结构与错误码不变，
  仅新增“请求体 > 64 KiB 的 session 信息请求返回 InvalidParam”这一输入边界）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes（加固项：为匿名 session 信息
  接口新增 64 KiB 请求体上限，削减超大体/chunked 请求导致的 nginx 临时存储与
  服务端内存消耗；属已确认的 standard 交付内容，未新增暴露面）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes（docker nginx 仅对 session 信息路由收紧到 64 KiB，上传/
  下载与其它位置行为不变；无新增依赖、无锁文件变化）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server --test api_integration request_body_over_64k`
  2/2 通过（038 登录超限回归 + 045 新增用例）；
  `cargo test -p filehub-server --test api_integration session` 4/4 通过
  （新增用例 + `api_login_session_and_token_flow` + 两条 refresh 会话用例）。
- Result: pass
- Residual risk or follow-up: 本环境未安装 nginx，`nginx -t` 未执行；模板改动为
  标准指令级配置，需在 Docker 构建/部署冒烟测试时验证精确匹配块与反代头；
  工作树中 025-044 等在制内容未触碰，未跑全量测试套件（非本任务范围）。
