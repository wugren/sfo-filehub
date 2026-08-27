task_manifest: task.yaml
status: approved

# 修复高危：/account/get_account_info_of_session 无界读取请求体

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳提案
  （get_account_info_of_session 请求体最大 64 KiB：Content-Length 预检 + 实际
  流量有界读取 + nginx 精确匹配路由 64k 限制）并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 与 038 同类的「未认证接口请求体有界读取」安全加固 bugfix：应用层需
    Content-Length 预检 + 实际流量有界读取，nginx 精确匹配路由限制，并新增
    HTTP 级回归验证固定长度与 chunked 两条绕过路径；
  - 不满足 trivial：改变认证前输入边界（安全/边界行为变更），且需要确定性
    回归验证；与 026/036/037/038 等同类 body-bound 任务沿用同一 standard 判定；
  - 未触发 high-risk：不新增公开 API 字段/协议、不涉及数据 schema/迁移、
    无发布/兼容/回滚/跨项目边界变化；直接依赖已由 038 加入
    （http-body-util、serde_json），本任务无新增依赖；nginx 仅对
    get_account_info_of_session 路由增加 64k 上游限制，上传/下载位置不受影响。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，确认采纳本提案（
  get_account_info_of_session 请求体最大 64 KiB，含 Content-Length 预检、
  实际流量有界读取与 nginx 路由 64k 限制）并接受建议的 standard 层级。

## Background and Goal

- 现象（评审高危项）：`POST /account/get_account_info_of_session` 在校验
  session 前直接调用无界 `req.body_json()`
  （`third_party/sfo-account/src/account_server.rs`），sfo-http 0.8 会把整个
  body 无上限累积进 `BytesMut`；Docker 侧 nginx 的 `client_max_body_size 0`
  又作用于整个 `/account/` 前缀（`docker/nginx.conf`），038 只对
  `/account/login` 加了 64 KiB 上限。未登录攻击者仍可对
  get_account_info_of_session 发送超大或持续 chunked body，消耗 nginx 临时
  存储与服务端内存。
- 目标：对 `/account/get_account_info_of_session` 施加与登录相同的 64 KiB
  请求体上限——先按 Content-Length 预检，再用 `take_http_body()` 有界流式
  读取（累计超过 65536 字节立即拒绝）；nginx 增加该路由精确匹配 64k 限制。

## Scope

### In scope

- `third_party/sfo-account/src/account_server.rs`：
  get_account_info_of_session handler 改为有界读取（复用/泛化 038 的
  `read_login_body()` 读取逻辑），超限错误沿用
  `AccountErrorCode::InvalidParam`；登录 handler 行为与错误文案保持现状；
- `docker/nginx.conf`：新增 `location = /account/get_account_info_of_session`
  精确匹配块，设置 `client_max_body_size 64k;` 并保留与 `^~ /account/`
  一致的反代头（仅该路由，`/api/v1/` 与 `^~ /account/` 其余路由不受影响）；
- `server/tests/api_integration.rs`：新增回归用例覆盖固定长度 Content-Length
  超限、无 Content-Length 的 chunked 实际流量超限、恰好 64 KiB 合法 session
  JSON 仍正常解析三条路径。

### Out of scope

- 不改 `/account/get_account_info` 与 `/account/refresh_session`（两者只读
  Authorization 头，不读取请求体）；
- 不改 sfo-http 源码，不为 sfo-http 增加本地 patch；
- 不改上传/下载的 `client_max_body_size 0` 与 `FH_MAX_ARCHIVE_BYTES` 语义；
- 不改登录协议字段、错误码契约、`docs/api/v1-contract.md` 或客户端；
- 不触碰 025-044 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 超限错误仍走 AccountServer 既有 `InvalidParam` 结果序列化路径，HTTP 响应
  结构不变；
- 有界读取基于 `sfo_http::http_server::Request::take_http_body` trait，
  actix 与 hyper 后端同时受约束；
- 复用 038 的读取实现时保持登录路由的拒绝消息不变化，避免既有登录用例回归。

## Requirement Review

- 需求合理：该接口只需一个小 JSON 对象 `{"session": "..."}`，64 KiB 上限远
  高于合法使用，可直接封住与 038 同源的未认证内存/临时存储消耗；
- 方向选择：应用层沿用 038 已验证的「Content-Length 预检 + 有界流读取」双重
  约束（覆盖 chunked 无 Content-Length 场景），nginx 对该路由精确匹配 64k，
  让 Docker 部署在代理缓冲前直接拒绝超大体；
- 材料风险/权衡：handler 由 `body_json()` 改为有界读取 + `serde_json` 解析，
  行为等价；读取函数若泛化，需保证登录错误文案不变；nginx 精确匹配块与 038
  同在 `^~ /account/` 之前按精确匹配优先级生效；
- 待确认问题：无。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-session-body-limit | get_account_info_of_session 请求体最大 64 KiB：Content-Length > 65536 或实际累计读取 > 65536 均拒绝（InvalidParam） | 仅 sfo-account 的 session-info handler；错误走既有 JSON 结果结构 | 有界读取替代 `body_json()`，等价解析行为，登录路由文案与行为不变 | 65 KiB（固定长度与 chunked 两种形态）请求返回 InvalidParam；恰好 64 KiB 合法 session JSON 仍进入解签 | 不限制其它路由，不修改 sfo-http |
| P-002 | fh-session-body-limit-nginx | docker nginx 对 `location = /account/get_account_info_of_session` 设置 `client_max_body_size 64k`，保留相同反代头 | 仅 docker/nginx.conf 该精确匹配块 | 代理缓冲前直接拒绝超大请求；`/api/v1/` 与其它 account 路由不受影响 | 精确匹配块含 64k 与全部 proxy_set_header | 不把 64k 上限扩到上传/下载或其它路由 |
| P-003 | fh-session-body-limit-tests | HTTP 级回归：固定长度 65 KiB 与流式 chunked 超限 get_account_info_of_session 均返回非 0 err，64 KiB 边界合法 session 不受限 | server/tests/api_integration.rs session-info 用例区 | 覆盖 Content-Length 与真实流量两条边界路径 | 新增用例通过；既有登录/账号集成用例不回归 | 不新增 sfo-account 单测或 mock Request 基建 |

## Success Criteria

- 65 KiB（含 65537 字节）固定长度与 chunked 的 get_account_info_of_session
  请求均被拒绝，返回 `AccountErrorCode::InvalidParam`（err != 0），过程中不
  累积超过上限的请求体；
- 恰好 64 KiB 的合法 session JSON 仍进入解签（有效 session 返回 err 0；
  无效 session 返回 SessionInvalid 而非超限错误）；
- 既有登录超限回归（`login_rejects_request_body_over_64k`）与账号集成用例
  继续通过，登录拒绝消息不变；
- `cargo check -p filehub-server` 与相关定向集成测试通过；
- docker nginx 模板中 session-info 路由 64k 限制与反代头完整；上传/下载位置
  未改动；
- 按 standard 流程产出 `docs/changes/045-session-body-limit.md` 与任务包
  `completion-report.md`（中文），并经 lower-tier-check 校验。

## Risks

- 读取函数若泛化（`read_login_body` 改为共享有界读取），登录 handler 与测试
  必须同步回归，确认拒绝消息不变化；若共享时消息必须变化，则保留登录侧原
  文案并仅新增 session 侧文案；
- 本环境未安装 nginx，`nginx -t` 无法本地执行；模板为标准指令级配置，需在
  Docker 构建/部署冒烟时验证精确匹配块与反代头（同 038 F-2 残余风险）；
- `third_party/sfo-account/` 整体仍为仓库内未提交的第三方导入（038 同现状），
  本任务只在其 handler 内做最小改动；sfo-account 上游同步时需携带该安全改动；
- 工作树存在大量未提交的在制改动（025-044 等），本任务只修改提案列出文件，
  不运行仓库级格式化；尾部验证聚焦 session-info/登录/账号相关用例。
