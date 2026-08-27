# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/046-login-hardening.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `third_party/sfo-account`：登录失败统一为 `InvalidPassword`（err=10）+
    固定中文消息 `用户名或密码错误`；账号缺失分支经 `LoginPasswordVerifier`
    的 `verify_dummy` 执行 cost=12 等成本伪校验；登录 handler 在读取请求体
    前调用新增 `LoginRateLimiter` seam 并新增 `TooManyRequests`（err=11）；
  - `server/src/account/store.rs`：`FilehubPasswordVerifier` 对真实 hash 与
    dummy hash 都经 `tokio::task::spawn_blocking` 执行 bcrypt，不再占用
    async worker；`server/src/account/rate_limit.rs`（新增）提供固定窗口
    按 IP 限流器（`X-Real-IP` → peer/remote 归一化纯 IP）；
  - 配置与部署：`HttpConfigSeed` 新增 `login_rate_limit_per_minute`（默认 30，
    0 关闭）与 `login_rate_limit_window_secs`（默认 60）；`filehub-server.json`
    与 `docker/entrypoint.sh`（`FH_LOGIN_RATE_LIMIT_*`）同步；`docker/nginx.conf`
    新增 `limit_req_zone` + 登录 location 的 `limit_req burst nodelay` + 429；
  - 测试：单元覆盖统一失败、dummy hash 成本、限流窗口/key 隔离/滚动恢复；
    集成覆盖 HTTP 级未知账号与错密码响应逐字节一致、超配额 err=11 且其它
    匿名账号接口不受限流；契约文档 `docs/api/v1-contract.md` 与
    `docker/README.md` 同步更新。
- Handoff: `cargo check -p filehub-server`、`cargo check -p filehub-cli --tests`
  通过；`--test unit_tests` 54/54 通过；`--test api_integration -- login`
  4/4 通过（含 038 登录体上限回归）；`lower-tier-check.py pre-edit/completion`
  通过；nginx 语法留待 Docker 冒烟（环境无 nginx，同 038/045 残余）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-login-credential-uniform | 登录失败统一 err=10 与固定消息；账号缺失执行 cost=12 dummy bcrypt；校验移出 async worker | proposal.md P-001 | `account_manager.rs` login 双分支统一 + `LoginPasswordVerifier` seam；`store.rs` `FilehubPasswordVerifier`（spawn_blocking）；单元用例 `login_failure_is_uniform_...` + `login_dummy_hash_matches_...` 通过 | 匹配 | pass |
| fh-login-rate-limit-app | 登录 handler 超限返回 err=11；默认 30 次/60 秒/IP，0 关闭 | proposal.md P-002 | `account_server.rs` 限流 seam + key 归一化；`rate_limit.rs` 固定窗口实现；config/entrypoint/filehub-server.json 字段与默认值；集成用例超配额 err=11、其它接口不受限流 | 匹配 | pass |
| fh-login-nginx-limit | nginx `limit_req`（5r/s、burst 20、429）仅作用于 `location = /account/login` | proposal.md P-003 | `docker/nginx.conf` zone + location 指令；`docker/README.md` 更新 | 匹配（语法留 Docker 冒烟） | pass |
| fh-login-hardening-tests | 统一失败响应一致、dummy hash 成本、限流 N+1 拒绝/窗口恢复/key 隔离、既有体上限用例不回归 | proposal.md P-004 | `server/tests/unit/rate_limit.rs`、`unit/account.rs` 新增用例 + `api_integration.rs` 两个新用例；定向与全量 unit 通过 | 匹配 | pass |
| fh-login-hardening-contract-docs | api 契约记录失败与限流语义；standard change record 与完成报告中文 | proposal.md P-005 | `docs/api/v1-contract.md` 新增小节；`docs/changes/046-login-hardening.md` 与本文档为中文 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|--------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `login` 的账号查找/校验两分支、`verify_dummy` 与 `verify` 的 spawn_blocking 路径、限流 allow/deny 计数、key 归一化（SocketAddr/IP 解析）、`register_server` 委托与新增方法签名 | 反向推演：未知账号是否仍可能提前返回（无——两分支都先等 bcrypt 完成再统一报错）；dummy hash 是否真为成本 12 的有效 bcrypt（单元用例解析 HashParts 并断言 cost）；限流是否在 body 读取前生效（是，超限请求不消耗 body/内存）；peer 端口变化是否绕过限流（已归一化为纯 IP，集成用例实测同一 key 第 3 次拒绝）；`spawn_blocking` JoinError 是否泄露信息（unwrap_or(false) 失败关闭） | 无绕过：统一失败与限流路径行为符合提案；唯一残余为配置成本 ≠12 的旧 hash 计时差（F-2）与直连伪造 X-Real-IP（F-1），均非默认部署路径 | pass |
| boundaries-and-failure-paths | 200 信封 err 10/11 边界、窗口滚动边界、key 最大 10000 修剪、`login_rate_limit_per_minute=0` 关闭、window_secs=0 钳位为 1、无关路由（get_account_info_of_session）不被限流 | 挑战：窗口滚动后配额恢复（1s 窗口 sleep 1.1s 用例）；超限后其它账号接口仍可用（集成用例断言 err=5 非 11）；配置 0 时 AppState 不注入限流器；`X-Real-IP` 缺失时回退 peer 且端口被剥离；IPv6 SocketAddr 经 std parse 归一化 | 边界无缺口：配额恢复、key 隔离、配置开关均被测试覆盖；窗口 0 钳位为 1 秒属防御性处理并在 change record 中可见 | pass |
| regression-and-side-effects | 038 登录体上限断言、`api_login_session_and_token_flow`、refresh 会话用例、vendored crate 其余构造器（new/new_eddsa/带 session config）默认行为、CLI e2e `HttpConfigSeed` 构造、web/CLI 不区分 err 9/10、Cargo.lock | 排查既有调用方是否依赖旧 err=9/账号名消息（无；测试仅断言 is_err / 契约桩消息）；`Account` trait 增加 `Sync` 是否破坏其它 impl（仓库内仅 FilehubAccount 与 vendored TestAccount，均满足）；vested crate inline 测试能否在仓库 canonical 入口执行（不能，同 044 既有 gap，本次未改其 manifest）；新增依赖（无，Cargo.lock 未变） | 无回归：登录/会话/体上限定向用例 4/4 与全量 unit 54/54 通过；CLI 测试编译通过；vendored inline 测试不可经本仓库入口执行属继承 gap（F-3） | pass |

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；`cargo check -p
  filehub-cli --tests` 通过；`cargo test -p filehub-server --test unit_tests`
  54/54 通过；`cargo test -p filehub-server --test api_integration -- login`
  4/4 通过；`docker/entrypoint.sh` `bash -n` 与 jq 生成表达式冒烟通过；
  `lower-tier-check.py --profile pre-edit` 与 `--profile completion` 通过
- Result: pass
- Exception reason: 全量 api_integration / 其余在制任务测试未跑（工作树存在
  025-045 等在制未收尾内容，属其负责）；nginx `nginx -t` 未执行（本机无
  nginx，同 038/045 记录为交付后 Docker 冒烟项）。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `login_rate_key` 优先信任 `X-Real-IP` | 绕过 nginx 直连暴露 server 时，攻击者可伪造 `X-Real-IP` 规避应用层限流；容器官方部署 server 仅监听 127.0.0.1、全部流量经 nginx，且 nginx 层 `limit_req` 按真实 `$binary_remote_addr` 计数 | no |
| F-2 | low | `seed_user` 接受 cost 4..=31 的 `password_hash` | 若旧配置种子的 bcrypt cost ≠12，其错误密码响应与 cost=12 dummy 校验仍有时差，可被用于存在性判断；默认（DEFAULT_COST=12）无此差异，建议后续收紧种子 hash 成本校验 | no |
| F-3 | none | 044 既有结论 | vendored sfo-account 的 inline 测试不经本仓库 canonical 入口执行（需要改其 manifest，超出本任务）；本次公开 API 行为由 server 端单元/集成用例覆盖 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 批准提案的 5 个 change_id 全部落地并经定向验证：登录失败响应统一、
  dummy bcrypt cost=12 等成本校验、bcrypt 移出 async worker、应用层 + nginx
  双层限流、契约与文档同步；独立缺陷复核未发现阻塞问题，F-1/F-2 属已记录
  的非默认部署/非默认配置残余，F-3 为继承测试 gap。
