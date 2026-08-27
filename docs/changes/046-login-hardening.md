# 登录加固：统一失败语义、等成本 bcrypt、移出 async worker 与双层登录限流

- Status: complete
- Owner module: filehub（sfo-account 登录链路 + filehub-server 校验/限流装配 +
  docker nginx 模板 + 测试与契约文档）
- Task manifest: `docs/versions/v0.1/modules/filehub/046-login-hardening/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/046-login-hardening/proposal.md`
- Affected paths:
  - `third_party/sfo-account/src/account_manager.rs`、
    `third_party/sfo-account/src/account_server.rs`、
    `third_party/sfo-account/src/errors.rs`
  - `server/src/account/store.rs`、`server/src/account/mod.rs`、
    `server/src/account/http.rs`、`server/src/account/rate_limit.rs`（新增）、
    `server/src/model/config.rs`、`server/src/http/mod.rs`
  - `filehub-server.json`、`docker/entrypoint.sh`、`docker/nginx.conf`、
    `docker/README.md`
  - `server/tests/common/mod.rs`、`server/tests/unit/mod.rs`、
    `server/tests/unit/account.rs`、`server/tests/unit/rate_limit.rs`（新增）、
    `server/tests/api_integration.rs`、`cli/tests/e2e_cli_server.rs`
  - `docs/api/v1-contract.md`、`docs/changes/046-login-hardening.md`、
    `docs/versions/v0.1/modules/filehub/046-login-hardening/completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 统一失败语义（`fh-login-credential-uniform`）：
  `DefaultAccountManager::login` 不再区分账号不存在与密码错误——两个分支统一
  返回 `InvalidPassword`（err=10）与固定消息 `用户名或密码错误`（不包含
  用户名）；账号缺失分支执行 cost=12 的固定 dummy bcrypt `verify`，实现与
  真实校验同成本。新增 `LoginPasswordVerifier` async seam：`new()` 等旧构造器
  保留内置同步校验器（vendored crate 自身行为不变），filehub-server 经
  `new_with_login_verifier` 注入生产校验器；
- 移出 async worker（`fh-login-credential-uniform`）：
  `FilehubPasswordVerifier`（server crate）对真实账号 hash 与 dummy hash 都
  经 `tokio::task::spawn_blocking` 执行 `bcrypt::verify`，bcrypt 不再占用
  sfo-http 的 async worker；`Account` trait 增加 `Sync` 约束以支撑 async
  校验 seam；
- 应用层限流（`fh-login-rate-limit-app`）：sfo-account 登录 handler 在读取
  请求体前调用可注入的 `LoginRateLimiter`（新增
  `register_server_with_login_rate_limiter`，原 `register_server` 委托
  None）；filehub-server 提供 `FixedWindowLoginLimiter`（内存固定窗口，
  key 取 `X-Real-IP` → `peer_addr`/`remote` 并归一化为纯 IP，杜绝同客户端
  不同临时端口绕过）；`AccountErrorCode` 新增尾随变体 `TooManyRequests`
  （err=11）；`HttpConfigSeed` 新增默认字段
  `login_rate_limit_per_minute=30` / `login_rate_limit_window_secs=60`
  （前者 0 关闭），`filehub-server.json` 与 `docker/entrypoint.sh`
  （`FH_LOGIN_RATE_LIMIT_*`）同步生成；
- nginx 限流（`fh-login-nginx-limit`）：`docker/nginx.conf` 模板 http 上下文
  新增 `limit_req_zone $binary_remote_addr zone=fh_login:10m rate=5r/s;`，
  `location = /account/login` 内 `limit_req zone=fh_login burst=20 nodelay;` +
  `limit_req_status 429`，与应用层构成双层防护；上传/下载与其它路由不变；
- 测试与文档（`fh-login-hardening-tests`、`fh-login-hardening-contract-docs`）：
  单元测试覆盖统一失败 code/msg、dummy hash 成本、限流窗口/key 隔离/滚动恢复；
  集成测试覆盖 HTTP 级未知账号与错密码响应完全一致、超配额返回 err=11 且
  其它账户接口不受限流；`docs/api/v1-contract.md` 记录失败/限流语义，
  `docker/README.md` 更新环境变量与"登录限流已在镜像内实现"的说明。

## Risk Screen

- Public contract, protocol, or CLI change: yes（登录失败 err/msg 统一为
  10 + `用户名或密码错误`，新增 err=11 限流语义；`docs/api/v1-contract.md`
  同步记录；web/CLI 客户端只展示消息不区分旧 err 9/10，无消费方回归）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes（修复项本身：统一失败 +
  等成本伪校验消除枚举侧信道、bcrypt 移出 async worker、双层限流；dummy
  hash 为公开固定 cost=12 哈希，不关联任何用户）
- Concurrency, lifecycle, or runtime integration change: yes（bcrypt 移入
  tokio blocking 池，有界；限流器 `Mutex<HashMap>` 临界区为微秒级且受
  `MAX_TRACKED_KEYS=10_000` 修剪约束）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes（无新增第三方依赖、Cargo.lock 未变；docker 部署新增
  登录限流默认值 30 次/分钟/IP 与 nginx 5r/s、burst 20，属可调生产默认；
  `FH_LOGIN_RATE_LIMIT_PER_MINUTE=0` 可关闭应用层限流）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no（登录 handler 仍由
  sfo-account 组装，filehub-server 只注入校验/限流策略；module 边界文档不变）

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；`cargo check
  -p filehub-cli --tests` 通过（e2e 配置构造编译校验）；`cargo test -p
  filehub-server --test unit_tests` 54/54 通过；`cargo test -p filehub-server
  --test api_integration -- login` 4/4 通过（含 038 体上限回归）；新增
  `login_rate_limit_rejects_over_quota_by_source_key` 与
  `login_failure_response_is_uniform_for_unknown_and_wrong_password` 定向用例
  通过；`docker/entrypoint.sh` 通过 `bash -n` 与 jq 生成表达式冒烟；
  `lower-tier-check.py pre-edit/completion` 通过。
- Result: pass
- Residual risk or follow-up:
  - 本机未安装 nginx，`nginx -t` 未执行，模板语法留待 Docker 构建/部署冒烟
    （与 038/045 同类残余）；
  - 直连暴露（绕过 nginx）时可伪造 `X-Real-IP` 规避应用层限流；容器官方
    部署 server 仅监听 127.0.0.1，全部流量经 nginx，残余仅限非官方直连场景；
  - 旧配置若用 cost≠12 的 `password_hash` 种子，该账号的错误密码响应仍与
    cost=12 dummy 校验存在计时差；默认（DEFAULT_COST=12）无此差异，
    建议后续收紧种子 hash 成本校验（见完成报告 F-2）。
