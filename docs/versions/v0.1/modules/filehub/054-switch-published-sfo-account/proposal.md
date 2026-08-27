---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-27
approved_content_sha256: b0fc38e9baabca452ca6bec09fc66efac5704e861f613fe631b33bd065d9789f
---

## Approval Record

- approver: user
- approval_date: 2026-08-27
- user_statement: 用户 2026-08-27 回复「按2方案实现，不管verify_dummy的问题」，
  确认采用 crates.io sfo-account 0.2.1 现状语义（未知账号 err=9、
  英文文案、不保留等成本伪校验），按 high-risk 全流程执行并适配测试/契约/
  配置/文档。

# 054-switch-published-sfo-account：删除 vendored sfo-account，改用 crates.io 发布包

Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 已确认触发 dependency/build graph 与 supply-chain 来源变更（删除
    `[patch.crates-io]` 与 `third_party/sfo-account`，Cargo.lock 切换为
    registry 0.2.1）；
  - 已确认触发 security/privacy 与公开契约影响：crates.io `sfo-account
    0.2.1`（GitHub HEAD 同码）的 `login()` 缺失账号分支直接返回
    `InvalidAccount`（err=9）且不执行等成本伪校验，与 046 login-hardening
    批准的「未知/错误密码统一 err=10、固定中文消息、消除枚举计时侧信道」
    不一致；此外限流消息变为英文，`decode_session` 的 sub 校验从「拒绝
    refresh」收紧为「只接受配置的 session_sub」；
  - 已确认触发 runtime/deployment 兼容：0.2.1 新增 HMAC session key 最短
    32 字节校验，而本项目测试配置 `test-session-key-please-change` 仅 30
    字节（`server/tests/common/mod.rs:60`、`cli/tests/e2e_cli_server.rs:82`），
    `DefaultAccountManager::new_with_login_verifier` 会直接 panic；当前
    `UsersConfig::session_key` 又没有启动期长度校验；
- 用户 2026-08-27 明确选择「按2方案实现，不管 verify_dummy 的问题」：
  接受 0.2.1 行为并适配，层级的最终确认为 high-risk，按完整生命周期执行。

## Background and Goal

现象：本仓库因上游 `sfo-account 0.2.0` 绑定 `sfo-http 0.7`，以 MIT 兼容
shim 形式在 `third_party/sfo-account/` 维护源码副本（未纳入 git 跟踪），
并在根 `Cargo.toml` 用 `[patch.crates-io] sfo-account = { path =
"third_party/sfo-account" }` 覆盖 crates.io 来源；038/044/045/046/047 等
任务直接修改过该副本。

用户请求：上游 `sfo-account` 已发布合并了本项目修改内容的新版本，可以删除
本地修改副本、直接使用发布的最新包。

现场核对（只读，2026-08-27）：

| 项目 | 本地 shim（038/044/045/046） | crates.io / GitHub 0.2.1 |
|------|------------------------------|--------------------------|
| 依赖面 | `sfo-http 0.8` + `http-body-util`、`serde_json` | 与本地一致，已发布 0.8 兼容版本 |
| 64 KiB 有界请求体（login / session info） | 已实现 | 已实现（文案一致） |
| `LoginPasswordVerifier` / `LoginRateLimiter` seam | 已实现 | 已实现 |
| refresh-only decode（拒绝 refresh 当 session） | 已实现 | 已等价实现（默认 sub 语义下行为一致） |
| 未知账号 + 错误密码统一 err=10、等成本伪校验 | 已实现（`verify_dummy` + `用户名或密码错误`） | **未合并**：未知账号直接返回 `InvalidAccount`（err=9），且不做伪校验；文案改英文 |
| 限流文案 | `登录尝试过于频繁，请稍后再试` | **英文**：`Too many login attempts; please try again later` |
| HMAC session key 长度 | 无下限 | 新增最短 32 字节；短 key 构造 `new*` 会 panic |
| `decode_session` sub | 仅拒绝等于 `refresh_sub` 的 token | 只接受等于配置 `session_sub` 的 token（默认 None；严格化） |

结论：发布包**合并了大部分**本地改动，但**没有合入 046 最关键的安全收窄
（未知账号等成本伪校验与统一 err=10/消息）**；GitHub 主干与 crates.io 0.2.1
一致，不是发布滞后。

## Scope

### In scope（用户当前请求的字面范围，方向确认后执行）

1. 依赖来源切换（`fh-sfo-account-published-source`）：
   - 删除根 `Cargo.toml` 的 `[patch.crates-io] sfo-account` 配置块及说明
     注释；
   - 删除 `third_party/sfo-account/` 本地副本（untracked，7 个文件）；
   - 更新 `Cargo.lock`：`sfo-account 0.2.1` 解析为 crates.io registry
     来源，锁文件不再包含 path-source 条目。
2. 一致性适配（`fh-sfo-account-conformance`，**按用户方向**收窄）：
   - 若接受 0.2.1 行为：更新 `server/tests/api_integration.rs`、
     `server/tests/unit/account.rs` 的拒绝语义/文案断言，更新
     `docs/api/v1-contract.md` 登录失败与限流契约；为
     `session_key` 增加最短 32 字节启动校验或改用返回 `AccountResult`
     的构造路径，并修正测试/e2e 密钥；
   - 若等待上游补齐：本项目不做代码/契约改动，仅保留决策记录。

### Out of scope

- 不修改 crates.io/GitHub 上 `sfo-account` 的源码或发布流程；
- 不做篇幅内的 sfo-account 重构、不加新接口、不动 vpn-server 等其它嵌入方；
- 不追溯改写 026/038/044/045/046/047 等历史 change record 与验收文档；
- 本次不轮换既有已签发 session（除非用户另行要求）。

## Requirement Review

- 请求方向合理：上游发布 0.2.1 已支持 `sfo-http 0.8` 并携带大部分 shim
  改动，长期应回到 crates.io 来源，避免在仓库内维护第三方源码副本。
- 关键风险/权衡：0.2.1 与本地 shim 在登录失败语义上不一致。按现状直接
  切换会在 `docs/api/v1-contract.md` 承诺的 `err=10 + 用户名或密码错误 +
  等成本伪校验` 上回退（未知账号回到 err=9、消息/响应时间可区分账号存在
  性），并导致现有集成/单元测试失败、30 字节测试密钥启动 panic。
- 建议方向：**先保持 shim，待上游把 046 的未知账号分支合并并发布新版本
  后再切换**；或与上游确认这是有意的行为变更后，再在本仓库按新契约适配并
  显式接受账号枚举风险。不建议未确认语义差异就删除副本。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | non_goal | success_evidence |
|-------------|-----------|-------------|----------|----------|---------|-----------------|
| P-001 | fh-sfo-account-published-source | 删除根 Cargo.toml 的 `[patch.crates-io] sfo-account` 与 `third_party/sfo-account/`，Cargo.lock 将 sfo-account 解析为 crates.io registry 0.2.1 | 仅本仓库依赖来源；不发布/修改上游 crate，不影响 vpn-server 等其它嵌入方 | 以 crates.io 校验和取代本地 MIT shim，构建可复现性改由锁文件与 registry 保证 | 不重写 026/038/044/045/046/047 等历史文档 | Cargo.lock 中 sfo-account 为 `0.2.1` + registry 来源且无 path 条目；third_party/sfo-account/ 不存在 |
| P-002 | fh-sfo-account-conformance | 按 0.2.1 语义收敛登录失败/限流契约、测试断言、session_key 最短 32 字节启动校验与测试密钥、模块/API 文档 | 仅改动 filehub-server 启动校验、测试 fixture 与账号会话相关文档；不新增路由/字段 | 接受未知账号 err=9、英文消息、无等成本伪校验的账号枚举信息面（用户已确认）；短 key 从静默可用改为启动拒绝 | 不在本仓库补回 verify_dummy 等成本伪校验；不轮换既有 session_key/session | 既有账号集成与单元测试全绿；v1-contract 描述 err=9/err=10/err=11 与英文文案；短 key 启动给出明确错误而非 panic |
| P-003 | fh-sfo-account-regression | 全量 workspace 回归 + 登录失败/限流/refresh 边界定向验证 | 验证范围为本任务变更面与账号/依赖消费方；不做其它模块功能评审 | 全量并行测试存在本工作树已知 502/EOF 干扰，以串行定向回归为判定依据 | 不借本任务扩大测试基建或修复无关失败 | `cargo test`（workspace 全目标）通过；test-run 任务级运行记录生成且为成功退出 |

## Success Criteria

- 用户确认方向后：
  - Cargo.lock 中 `sfo-account` 为 `0.2.1` registry 来源，无 path 依赖，
    `third_party/sfo-account/` 已删除（等待方向下不删除）；
  - `cargo test`（workspace）与既有 login/rate-limit/refresh 集成回归按
    确认后的契约全绿；
  - 登录失败语义（err 码/消息/伪校验或显式接受的替代行为）与
    `docs/api/v1-contract.md` 一致，且有独立缺陷发现记录；
  - 短 session_key 不再导致未说明 panic（校验或测试密钥修正）。

## Risks

| 风险 | 等级 | 说明 |
|------|------|------|
| 账号枚举回归（安全） | 高 | 0.2.1 未知账号无伪校验且 err/message 不同；直接切换削弱 046 审批交付的安全边界 |
| 启动 panic（部署） | 中 | `session_key` < 32 字节时 0.2.1 `new*` 直接 panic；当前配置解析无校验 |
| 公开契约漂移 | 中 | err=10/消息与 `docs/api/v1-contract.md` 不一致；web/cli/既有部署消费方可感知 |
| supply-chain / 构建图 | 中 | 删除本地 MIT shim 后以 crates.io 校验和为准；需锁文件与干净构建回归 |
| 其它嵌入方兼容 | 低 | 仅本项目切换，不下发 vpn-server 等其它 crate 消费者 |

## Unresolved Questions

1. 是否接受「直接使用 0.2.1 当前行为」（未知账号 err=9、英文文案、无等成本
   伪校验），并按此适配测试/契约/配置？还是先等上游补齐 046 伪校验后发布
   新版本再切换？
2. 若切换，是否同意本次一并增加 `session_key` 最短 32 字节的配置启动校验
   并修正测试/e2e 密钥（属于依赖升级的必要兼容收口）？
