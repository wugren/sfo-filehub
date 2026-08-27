# 匿名登录接口 /account/login 请求体上限 64 KiB

- Status: complete
- Owner module: filehub（sfo-account 登录 handler + docker nginx 模板 + HTTP 集成测试）
- Task manifest: `docs/versions/v0.1/modules/filehub/038-login-body-limit/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/038-login-body-limit/proposal.md`
- Affected paths: `third_party/sfo-account/src/account_server.rs`、
  `third_party/sfo-account/Cargo.toml`、`docker/nginx.conf`、
  `server/tests/api_integration.rs`、`docs/changes/038-login-body-limit.md`、
  `docs/versions/v0.1/modules/filehub/038-login-body-limit/completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷：`POST /account/login` 在认证前调用 sfo-http 的 `body_json()`，而
  sfo-http 0.8 会把整个 body 无上限累积进 `BytesMut`
  （`actix_server/endpoint.rs` 的 `body_bytes`/`body_string`）；Docker nginx
  又设 `client_max_body_size 0`，未登录攻击者可发送超大/无限 chunked 请求消耗
  nginx 临时存储与服务端内存。
- 修复（应用层）：登录 handler 改为 `read_login_body()`——先解析
  `Content-Length` 做预检（声明 > 65536 字节直接拒绝），再用
  `take_http_body()` 流式读取并累计，超过 64 KiB 立即拒绝；超限错误沿用
  `AccountErrorCode::InvalidParam`。该读取基于 `sfo_http::Request::take_http_body`
  trait，actix 与 hyper 后端同时受约束。
- 修复（代理层）：`docker/nginx.conf` 新增 `location = /account/login` 精确匹配
  块并设 `client_max_body_size 64k`，Docker 部署在代理缓冲前直接拒绝超大登录体；
  `/api/v1/` 上传/下载及全局 0 不受影响。
- 依赖：`third_party/sfo-account` 补充 `http-body-util`、`serde_json` 直接依赖
  （两部分包均已存在于 Cargo.lock 解析图中，无新增包）；登录 JSON 解析行为与
  原 `body_json()` 等价。
- 测试：`server/tests/api_integration.rs` 新增
  `login_rejects_request_body_over_64k`，覆盖 64 KiB 边界（合法 JSON 仍进入账号
  校验）、固定长度 65 KiB Content-Length 预检、无 Content-Length 的流式 chunked
  超限三条路径。

## Risk Screen

- Public contract, protocol, or CLI change: no（请求/响应结构与错误码不变，
  仅新增“请求体 > 64 KiB 的登录请求返回 InvalidParam”这一输入边界）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes（加固项：为未认证登录接口新增
  64 KiB 请求体上限，削减超大体/chunked 请求导致的 nginx 临时存储与服务端内存
  消耗；属已确认的 standard 交付内容，未新增暴露面）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes（sfo-account 新增两个已在锁文件解析图中的直接依赖；docker
  nginx 仅对 /account/login 收紧到 64 KiB，上传/下载与其它位置行为不变）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server --test api_integration login` 2/2 通过
  （`login_rejects_request_body_over_64k` 新增用例 + `api_login_session_and_token_flow`
  既有登录流回归）
- Result: pass
- Residual risk or follow-up: 本环境未安装 nginx，`nginx -t` 未执行；模板改动为
  标准指令级配置，需在 Docker 构建/部署冒烟时验证；其余 account JSON 路由
  （get_account_info_of_session、refresh_session）应用层仍无上限，系用户收窄
  范围后的明确非目标；工作树中 025-037 等在制内容未触碰。
