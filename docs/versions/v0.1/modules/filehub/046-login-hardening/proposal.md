---
task_manifest: task.yaml
status: approved
---

# 046-login-hardening：登录接口账号枚举与 CPU 拒绝服务修复（中危）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳提案
  （统一登录失败语义、dummy bcrypt 等成本校验、spawn_blocking 移出 async
  worker、应用层 + nginx 双层登录限流）并接受建议的 standard 层级。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，确认采纳本提案
  （登录失败统一 err=10 与固定中文文案、未知账号 cost=12 dummy bcrypt 校验、
  bcrypt 移出 async worker、应用层固定窗口限流 + nginx limit_req 双层限流）
  并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 防御性安全修复，改变"登录失败响应"的公开契约语义（错误码/消息不再区分
    账号是否存在）并新增登录限流边界，属于安全/信任边界行为变更；不满足
    trivial（trivial 要求无安全边界影响）；
  - 与已完成的同类安全加固任务（030 bcrypt 升级、038 登录请求体 64 KiB 上限、
    045 session 信息请求体上限）同构：单项目模块内的有界行为修复 + 定向回归
    验证 + nginx 部署配置，沿用 standard 判定；
  - 未触发 high-risk：不新增数据库 schema/迁移、不改变文件/项目/上传数据
    语义、不新增第三方依赖（bcrypt/tokio/sqlx 已存在）、无发布/回滚协调或
    跨项目边界变化；错误码变化是本次修复的目标本身，且客户端只展示不区分，
    不构成要求 high-risk 的"兼容性需要协调"；已有 045 等任务证明本仓库对该
    类修复采用 standard 流程。

## Background and Goal

- 现象（安全评审中危项 #3）：登录接口可用作账号枚举与 CPU 拒绝服务。
  - 错误区分：`DefaultAccountManager::login` 对不存在账号返回
    `AccountErrorCode::InvalidAccount`（err=9）与消息 `account {} not found`，
    对密码错误返回 `InvalidPassword`（err=10）与
    `account {} password error`，账号是否存在可直接从响应区分
    （`third_party/sfo-account/src/account_manager.rs:310` 附近）；
  - 计时侧信道：不存在账号直接短路返回、跳过 bcrypt 校验，响应时间与密码
    错误明显不同，可离线比对响应时长枚举账号；
  - CPU 拒绝服务：bcrypt::verify 在 sfo-account 的 async handler 内同步执行
    （`server/src/account/store.rs` `FilehubAccount::verify_password`），持续
    并发登录请求会占满 HTTP worker 线程；应用层与 nginx 均无登录限流
    （`docker/nginx.conf` 全局 `client_max_body_size 0`、`location = /account/login`
    只有 64k 体上限）。
- 目标：三者一并修复——① 不存在账号与密码错误返回完全相同的错误码与消息
  （成功/失败时长也做等成本化，消除枚举侧信道）；② 把 bcrypt 校验移出
  async worker（tokio spawn_blocking）；③ 应用层与 nginx 双层登录限流。

## Scope

### In scope

- `third_party/sfo-account`（本仓库维护的兼容 shim，038/045 已直接改其源码）：
  - `account_manager.rs`：登录失败统一为一个错误码与固定消息（不重复账号名）；
    新增可注入的密码校验器（`LoginPasswordVerifier`）seam，真实账号与"账号
    不存在"分支都走校验器，由校验器负责等成本校验与移出 async worker；
    `DefaultAccountManager::new()` 保留旧的同步校验默认行为（vendored crate
    自身测试与其它嵌入方不受破坏），服务器通过新构造器注入落地校验器；
  - `account_server.rs`：登录 handler 在读取请求体前调用可注入的登录限流器
    seam（新增 `register_server_with_login_rate_limiter`，原注册方法委托
    None 保持兼容）；数据源端校验限流 key；
  - `errors.rs`：`AccountErrorCode` 新增 `TooManyRequests`（追加在枚举尾部，
    err=11，不改变既有编号）；
- `server/src/account/store.rs`：实现注入式校验器——真实账号与不存在账号
  分别以真实 hash / 固定 dummy bcrypt hash（cost=12，与 `bcrypt_hash` 的
  `DEFAULT_COST` 一致）执行 `bcrypt::verify`，并包在
  `tokio::task::spawn_blocking` 中，使 bcrypt 不再占用 async worker；
- `server/src/account/rate_limit.rs`（新增）：内存固定窗口按 IP 限流器
  （`HashMap<key,(window_start,count)>` + `Mutex`），key 优先取 nginx 写入的
  `X-Real-IP`，回退 `peer_addr`/`remote`；超限返回新的 `TooManyRequests`
  err=11 与固定消息，HTTP 仍走 sfo-http 200 信封（与账号接口既有错误风格一致）；
- `server/src/model/config.rs` + `filehub-server.json` + `docker/entrypoint.sh`：
  `HttpConfigSeed` 新增两个 serde-default 字段
  `login_rate_limit_per_minute`（默认 30，0=关闭）与
  `login_rate_limit_window_secs`（默认 60），入口脚本 jq 同步生成，便于部署
  调参与测试注入小窗口；
- `docker/nginx.conf`：新增 `limit_req_zone $binary_remote_addr
  zone=fh_login:10m rate=5r/s;` 与 `location = /account/login` 内
  `limit_req zone=fh_login burst=20 nodelay;`（并显式 `limit_req_status 429;`），
  在代理侧先于应用层拒绝高频登录；上传/下载与其它路由不受影响；
- 测试：`server/tests/unit/account.rs`（或新增 `rate_limit.rs`）与
  `server/tests/api_integration.rs` 新增——统一失败响应、dummy hash 成本、
  限流窗口行为、HTTP 级"不存在账号 vs 密码错误"响应字节一致、超限 err=11；
- 文档：`docs/api/v1-contract.md` 记录登录失败统一语义与 err=11；
  `docker/README.md` 更新"镜像内不实现限流"表述（登录限流已在镜像内实现，
  对外 HTTPS/防火墙仍建议前置网关处理）；standard 交付 `docs/changes/046-login-hardening.md`
  与任务包 `completion-report.md`（中文）。

### Out of scope

- 不删除 `InvalidAccount`/`InvalidPassword` 枚举变体（保留兼容，仅登录不再
  返回 `InvalidAccount`）；
- 不做按账号名（username-keyed）的限流/锁定：防止攻击者用目标账号名刷满
  配额造成针对特定账号的锁定拒绝服务；本任务只按请求来源 IP 限流；
- 不引入验证码（captcha 已在 crate 中预留但不启用）、不改登录协议字段、
  不改 HTTP 状态码体系（应用层继续 200 信封 + err 码）、不做 HTTPS/防火墙
  策略；
- 不触碰 `/account/get_account_info*`、`/account/refresh_session` 的错误码
  与行为；不触碰 025-045 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 登录 handler 仍由 sfo-account 的 `AccountServer` 组装（保持
  "filehub-server 不自写 login handler、直接导出 sfo-account 现役接口"的模块
  边界）；密码校验、校验成本与限流策略由 filehub-server 注入，产品策略留在
  产品代码，vendored crate 只提供 seam；
- 限流只发生在 `location = /account/login` 与应用层登录入口；`/api/v1/` 与
  上传/下载位置及全局 `client_max_body_size 0` 语义保持不变；
- 失败响应仍走既有 sfo-http `{err,msg,result}` 序列化路径；
  admin-web 登录契约桩已用 `用户名或密码错误` 作为统一失败文案，本次让真实
  服务端与该文案一致（err 码由 9/10 变为统一 10，契约桩不比对 err 数值）。

## Requirement Review

- 需求合理：评审中危项指认的三条路径（错误区分、计时侧信道、worker 占用）
  都可被直接关闭，且修复互相独立可验证；
- 方向选择：
  - 统一失败：两个分支都返回 `InvalidPassword`（err=10）与固定中文消息
    `用户名或密码错误`（与 admin-web 契约桩文案对齐；不包含用户名）；
  - 等成本：账号不存在分支用固定 dummy bcrypt hash（cost=12）执行同样的
    `bcrypt::verify`；dummy hash 无用户信息，公开不构成泄露；
  - 移出 worker：真实与 dummy 校验都进 `spawn_blocking`，async worker 只做
    DB 查询与 JWT 签发；`spawn_blocking` 有界线程池 + nginx 限流共同限制 CPU
    消耗；
  - 限流：应用层固定窗口按 IP（`X-Real-IP` → `peer_addr`），默认
    30 次/60 秒/IP；nginx `limit_req` 5r/s、burst 20，作为容器部署的
    权威首层；
- 材料风险/权衡：
  - 可信头选择：nginx 会把 `$remote_addr` 写进 `X-Real-IP`（覆盖客户端值），
    故应用层信任该头；容器内 server 只监听 127.0.0.1，直接访问只能到 nginx。
    若绕过 nginx 直连暴露的 server，攻击者可伪造 `X-Real-IP` 绕过应用层
    限流——因此本任务同时加 nginx 首层限流，并文档注明该残余；
  - 固定窗口 vs 令牌桶：固定窗口实现简单、无新增依赖、行为可测；窗口初期
    允许瞬时突发由 nginx burst 平滑，生产调参空间保留；
  - 不做基于响应时间差的自动化断言（测试易抖动），改用"响应字节一致 +
    dummy hash 与真实 hash 同 cost"的确定性验证；
- 待确认问题：无（见 Risks 中已列明的假设：统一文案用中文、默认 30/min/IP、
  新 err=11 走 200 信封；如需要可改英文文案或 429，答复时指出即可）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-login-credential-uniform | 登录失败统一：不存在账号与密码错误返回相同 err（10）与固定消息 `用户名或密码错误`；不存在账号执行与真实账号同成本（cost=12）的 dummy bcrypt 校验 | 仅 `DefaultAccountManager::login` 失败分支；handler 与错误信封结构不变 | 统一文案为中文并去掉用户名字段；seam 由服务器注入使 vendored crate 保持无 bcrypt 依赖 | manager.login 对 unknown 账号与错密码返回相同 code+msg；dummy 校验走 spawn_blocking 且 hash cost=12；既有登录成功用例不回归 | 不删除枚举变体；不改变成功响应 |
| P-002 | fh-login-rate-limit-app | 应用层登录限流：登录 handler 先查限流器，超限返回 err=11（TooManyRequests）+ 固定消息；默认 30 次/60 秒/IP，0 可关闭 | 仅 `/account/login` 应用入口；key 取 X-Real-IP、回退 peer_addr/remote | 固定窗口按 IP，窗口初期可突发（nginx 做平滑）；不做账号级限流避免锁定滥用 | 超出配额的下一次登录返回 err=11；不同 key 独立计数；恢复窗口后放行；配置缺省即生效 | 不做账号名限流、验证码、429 状态码 |
| P-003 | fh-login-nginx-limit | nginx 对 `location = /account/login` 加 `limit_req`（5r/s、burst 20、429），`limit_req_zone` 定义在模板 http 上下文内 | 仅该 location；上传/下载与其它 `/account/` 路由不变 | nginx 限流按 `$binary_remote_addr`，IPv6 前缀完整计数 | 模板含 zone、burst、429 状态与中文注释；`nginx -t` 语法在 Docker 构建冒烟中验证（本机无 nginx，见 Risks） | 不把限流扩展到其它路由 |
| P-004 | fh-login-hardening-tests | 新增回归：未知账号/错密码 HTTP 响应完全一致；dummy hash 成本与真实一致；限流 N+1 拒绝、窗口恢复、key 隔离；既有 038/045 体上限用例不回归 | server 单元 + 集成测试文件 | 不做现实计时断言（抖动），用等价响应与成本参数断言 | `cargo test -p filehub-server` 定向用例通过；既有登录/体上限用例不回归 | 不新增 vendored crate 单测（无法经本仓库 canonical 入口执行，沿用 044 结论） |
| P-005 | fh-login-hardening-contract-docs | `docs/api/v1-contract.md` 记录登录失败统一 err=10 文案与 err=11 限流语义；docker/README 更新限流已内置的说明 | 仅上述文档与标准 change record/完成报告 | 契约文档同步以中文记录本次语义收紧 | 文档与实现一致；standard change record 与 completion-report 通过 lower-tier-check | 不改其它接口的契约记录 |

## Success Criteria

- 不存在账号与密码错误的 `POST /account/login` 响应 err 与 msg 完全一致
  （HTTP 集成用例断言响应体逐字节相等），且消息固定为 `用户名或密码错误`；
- 不存在账号分支执行与真实分支同成本（bcrypt cost=12）的 dummy 校验，且
  校验运行在 `tokio::task::spawn_blocking`，不再占用 async worker；
- 应用层限流生效：同一 key 第 N+1 次登录返回 err=11（`TooManyRequests`）；
  不同 key 互不影响；窗口滚动后配额恢复；`login_rate_limit_per_minute: 0`
  可关闭；
- `docker/nginx.conf` 含 `limit_req_zone`、`limit_req burst nodelay` 与
  `limit_req_status 429` 且仅作用于 `location = /account/login`；
- `docs/api/v1-contract.md` 与 `docker/README.md` 同步更新；
- `cargo check -p filehub-server`、定向单元/集成测试与
  `harness/scripts/lower-tier-check.py --profile pre-edit|completion` 通过；
  `docker/entrypoint.sh` 的 jq 生成配置含新限流字段（无本机 nginx 时
  `nginx -t` 留待 Docker 构建冒烟，见 Risks）；
- 按 standard 流程产出 `docs/changes/046-login-hardening.md` 与任务包
  `completion-report.md`（中文），经 lower-tier-check 校验后移交。

## Risks

- 限流误伤：同一公网 IP（NAT/办公室出口）下多用户可能共用配额；默认 30/60s
  为管理台单人使用提供充足余量，若生产出现误伤可通过
  `login_rate_limit_per_minute` 调参；
- 直连暴露部署：绕过 nginx 直连 filehub-server 时可伪造 `X-Real-IP` 规避
  应用层限流；容器官方部署 server 仅监听 127.0.0.1、全部流量经 nginx，残余
  风险仅限非官方直接暴露场景，文档中注明；
- 分布式 IP 攻击仍可分摊 nginx 配额，`spawn_blocking` 池在极端并发下有界
  饱和；此为中危项的残余（与评审结论一致），不再触发更高风险流程；
- 本机未安装 nginx，`nginx -t` 无法本地执行；模板自上轮任务起沿用
  标准指令，语法留待 Docker 构建/部署冒烟验证；
- 工作树存在 025-045 等在制未提交改动：只读基线会在 pre-edit 阶段固化，
  本任务只改动上述 Scope 内文件，不触碰其它在制内容；
- 限流/错误语义为显式收紧的公共行为变更：`docs/api/v1-contract.md` 与
  change record 同步记录，已有测试与客户端均不依赖旧 err=9/账号名消息。
