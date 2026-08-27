---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-26
user_statement: 用户 2026-08-26 回复「修复吧」，确认采纳本提案并按建议的 high-risk 层级全流程执行：refresh token 只能用于 refresh，不能在 decode_session 映射为用户身份。
---

# refresh token 只能用于 refresh，不得冒充普通 session 访问用户接口

Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Proposal and tier confirmation: 用户 2026-08-26 回复「修复吧」，确认采纳本
  提案（refresh token 只能用于 refresh，decode_session 拒绝 refresh 类型）
  并接受建议的 high-risk 层级与全流程。
- Tier rationale / triggered boundaries:
  - 这是明确的安全边界缺陷（用户标注「高危」）：有效期 30 天的 refresh
    token 与普通 session 使用同一签名密钥，仅靠 `sub="refresh"` 区分，而
    `decode_session()` 只验签和检查过期、不拒绝 refresh 类型，随后认证桥
    （`server/src/http/auth.rs`）把解码结果直接映射为 `Principal::User`，
    导致 refresh token 可以访问项目、Token 管理等全部用户接口；
  - 修复会改变「到底什么凭据算 session」的认证语义并新增回归测试，属于
    生产安全/认证边界变更，不满足 trivial 对「无 security 实质影响」的
    界定；也不满足 standard「无 material security boundary impact」的默认
    前提，因此按高风险的 proposal -> design -> implementation -> testing
    -> acceptance 全流程执行。

## Background and Goal

### 现象（评审高危项原文）

- refresh token 与普通 session 使用同一签名密钥，仅通过 `sub="refresh"`
  区分；
- `decode_session()` 只检查签名和过期时间，没有拒绝 refresh 类型；
- 随后认证桥直接把它映射成用户身份（`server/src/http/auth.rs:16`）；
- 因此有效期 30 天的 refresh token 可以访问项目、Token 管理等所有用户
  接口，而不仅用于续期。

### 代码证据（当前工作树逐行核对）

1. `third_party/sfo-account/src/account_manager.rs:277-299`
   `generate_session`：普通 session 默认不带 `sub`、有效 1 小时；
   refresh session 固定 `sub(self.session_config.refresh_sub)`（默认
   `"refresh"`），有效期默认 30 天；
2. `account_manager.rs:340-356` `decode_session`：`JsonWebToken::
   decode_payload` 验签后只检查 `is_expire()`，未区分 `token.sub`，直接
   `Ok(token.data)` 返回用户；
3. `account_manager.rs:324-338` `refresh_session`：入口已校验
   `token.sub == refresh_sub`，即正常 session 不能换发，但该校验只保护
   refresh 端点自身；
4. `server/src/http/auth.rs:16-22` `SessionAuthWrapper::decode_user`：
   调用 `account.decode_session(bearer)` 成功后直接把 `account.id` 映射为
   用户；`server/src/account/authn.rs:9` `try_user_principal` 同路径；
5. `third_party/sfo-account/src/account_server.rs:122-148`：
   `/account/get_account_info_of_session` 与 `/account/get_account_info`
   同样只依赖 `decode_session`，也一并受影响；
6. token 凭据路径不受影响：token JWT 由 `tokens::resolve` 用每 token
   独立验签公钥校验，refresh token（HMAC session 密钥签发）无法通过。

### 目标

refresh token 只能调用 `/account/refresh_session` 完成续期，不能作为
普通 session 访问任何用户身份接口（`/account/get_account_info*`、
`/api/v1/*` 等）。正常 session 的解码、过期语义与 refresh 续期流程保持
原有行为不变。

## Scope

### In scope

1. `fh-refresh-decoder-reject`
   (`third_party/sfo-account/src/account_manager.rs`)
   - 在 `DefaultAccountManager::decode_session` 完成验签与过期检查后，
     增加 refresh 类型拒绝：当 `token.sub.as_deref() ==
     Some(self.session_config.refresh_sub.as_str())` 时返回
     `AccountErrorCode::SessionInvalid`（错误信息说明 refresh session
     不能作为访问 session 使用）；
   - `SessionConfig::validate`（已存在，`account_manager.rs:75`）保证
     `session_sub != refresh_sub`，默认配置 `session_sub = None` 意味着
     普通 session 不带 `sub`，因此该判断不会误伤正常 session；refresh
     session 是 `refresh_sub` 的唯一签发来源；
   - 该修复位于唯一解码收口，`server/src/http/auth.rs`、
     `server/src/account/authn.rs`、`account_server.rs` 的
     `/account/get_account_info*` 三个消费方自动同时生效，
     `server/src/account/mod.rs:58` 无需改动。
2. `fh-refresh-regression`（回归测试）
   - `third_party/sfo-account/src/account_manager.rs` 单测：`decode_session`
     拒绝 refresh token（`SessionInvalid`）、接受普通 session；追加
     `refresh_session` 仍只接受 refresh token 的既有断言不变；
   - `server/tests/unit/account.rs`：`seeds_users_and_logins` 增加
     `decode_session(&refresh)` 必须失败的断言（正常 session 解码仍成功）；
   - `server/tests/api_integration.rs`：新增 API 层回归——用 refresh token
     作为 Bearer 访问用户接口（如 `GET /api/v1/projects` 与
     `GET /account/get_account_info`）必须被拒绝（受保护 API 返回 401；
     走 sfo-account 200 错误信封的接口 `err != 0`）；同时 `POST
     /account/refresh_session` 携带换发后的新 session 仍可访问用户接口
     （沿用既有 `session_refresh_rotates_and_new_session_works` 断言）。

### Out of scope

- 不修改 token 模块：token 凭据与 session 的验签路径与 claims 设计不变；
- 不新增 `typ` 等 JWT claim，不做 session 白名单/数据库会话表；当前
  `sub` 判别已能完整区分本仓库两种 session 类型；
- 不改变登录接口响应结构、session/refresh 有效期、refresh 轮换语义与
  CLI/admin-web 续期调用方式；
- 不复刻 `refresh_session` 的 sub 校验到别处，也不改动
  `server/src/http/auth.rs` 凭据分支结构；
- 不触碰本工作树中 025-043 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- account 是唯一签发/解码 session 与 refresh token 的模块；修复放在
  `sfo-account` 解码收口后，server 认证桥、`AccountServer` 账户信息接口
  与 permissions/tokens/projects 等依赖 `Principal::User` 的下游全部
  获得防线；
- `third_party/sfo-account` 是本仓库 Cargo `[patch.crates-io]` 的本地
  源码（根 `Cargo.toml:9`），本次直接修改 vendored 源码；上游
  发布支持 sfo-http 0.8 的新版本后，需回归确认该修复随 patch 对齐。

## Requirement Review

- 需求合理且与代码现状一致：评审指出的 `decode_session()` 缺少 refresh
  类型拒绝属实；`refresh_session` 入口的 sub 校验不能反向保护普通用户
  接口，认证桥又直接信任 `decode_session` 结果，构成实际的认证绕过。
- 已考虑的最小实现即拒绝方向：在解码收口按 `refresh_sub` 拒绝，而不是
  在 server 各路由逐个加判断，避免遗漏消费方；开关面只影响 refresh
  token 冒充行为的用户接口，正常登录/续期/换发链路不受影响。
- 已核对配置安全网：`SessionConfig::validate` 禁止 `session_sub` 与
  `refresh_sub` 相同，默认模式下普通 session 无 `sub`，因此修复不会把
  正常 session 误判为 refresh。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|---------|
| prop-044-refresh-decode | fh-refresh-decoder-reject | `DefaultAccountManager::decode_session` 必须拒绝 `sub == refresh_sub` 的 token（返回 `SessionInvalid`），使 refresh token 不能映射为用户身份；正常 session 解码、过期语义与 refresh 续期流程保持不变。 | 只改 `decode_session` 解码收口；不改签名、签发、refresh_session、认证桥结构、HTTP 契约与 token 凭据路径。 | 用 sub 判别而非新增 JWT claim：配置校验已保证 session_sub != refresh_sub，拒绝分支零误伤且改动最小。 | 单测：`decode_session(refresh_session)` 返回 `SessionInvalid`，`decode_session(session)` 返回用户；集成：refresh token 作为 Bearer 访问 `/api/v1/projects` 返回 401、`/account/get_account_info` 信封 `err != 0`。 | 不新增 JWT claim、session 白名单或数据库会话表；不改有效期与轮换语义。 |
| prop-044-refresh-regression | fh-refresh-regression | 为修复补齐回归测试：sfo-account 单测、server 单元与 API 集成均覆盖 refresh token 不能用作访问凭据；refresh 端点换发成功且换发后的新 session 照常访问用户接口。 | 测试面限定在 sfo-account 测试、server 单元与 API 集成；不新增测试基础设施与模块级套件改动。 | 在 vendored crate 处写测试需要新增独立测试文件并纳入 testplan 步骤；相对把断言塞进生产源码的既有 inline 单测，更容易被统一入口验证。 | 全部任务级测试通过（unit/dv/integration），既有 `session_refresh_rotates_and_new_session_works` 保持通过；canonical `test-run.py filehub/044-refresh-token-session-only all` 运行产物全绿。 | 不改 CLI/admin-web、mock 契约与既有测试断言。 |

## Success Criteria

- 单元：`decode_session(refresh_session)` 返回 `SessionInvalid`；
  `decode_session(session)` 仍返回用户；`refresh_session(session)` 仍被
  拒绝、`refresh_session(refresh_session)` 仍成功换发；
- 集成：refresh token 作为 Bearer 访问 `/api/v1/projects` 返回 401、
  `/account/get_account_info` 返回 `err != 0`；换发后的新 session 照常
  访问用户接口；
- 既有 `filehub` 单元/集成测试全部通过（canonical
  `harness/scripts/test-run.py filehub unit/integration`），且
  `session_refresh_rotates_and_new_session_works` 仍通过；
- 完成 high-risk 全流程：design/testing/acceptance 文档、生命周期收据与
  独立缺陷搜索报告齐备并经检查器验证。

## Risks

- 误伤面：刷新语义变化影响任何仍把 refresh token 当 session 使用的存量
  客户端；本仓库 CLI/admin-web 均只在 401 后调用 `/account/refresh_session`
  续期，不把 refresh 当访问凭据，风险低；CLI 集成 mock 仍会验证续期路径；
- vendored 依赖：`third_party/sfo-account` 为本地 patch 源码，修复不随
  crates.io 版本自动生效；需在本仓库持续维护，已在 out-of-scope 记录；
- 兼容性：不改 token 格式、签名算法、有效期与契约响应，无需数据迁移或
  存量 token 轮换（修复前已泄露的 refresh token 仍可续期直到过期，
  安全事件处置不属本任务代码范围，需在部署说明中提示轮换 session_key）。
